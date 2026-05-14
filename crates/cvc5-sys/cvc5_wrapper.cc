#include "cvc5_wrapper.h"

#include <cvc5/c/cvc5.h>
#include <cvc5/c/cvc5_parser.h>

#include <cstdlib>
#include <cstring>
#include <exception>
#include <sstream>
#include <string>

namespace {

char* copy_string(const std::string& value) {
  char* out = static_cast<char*>(std::malloc(value.size() + 1));
  if (out == nullptr) {
    return nullptr;
  }
  std::memcpy(out, value.c_str(), value.size() + 1);
  return out;
}

SoterCvc5SolveResult result(SoterCvc5Status status,
                            const std::string& diagnostics = {},
                            const std::string& model = {},
                            const std::string& unsat_core = {}) {
  return SoterCvc5SolveResult{
      status,
      model.empty() ? nullptr : copy_string(model),
      unsat_core.empty() ? nullptr : copy_string(unsat_core),
      diagnostics.empty() ? nullptr : copy_string(diagnostics),
  };
}

std::string cstr(const char* value) {
  return value == nullptr ? std::string{} : std::string(value);
}

SoterCvc5Status status_from_result(Cvc5Result cvc5_result,
                                   std::string* diagnostics) {
  if (cvc5_result_is_sat(cvc5_result)) {
    return SOTER_CVC5_STATUS_SAT;
  }
  if (cvc5_result_is_unsat(cvc5_result)) {
    return SOTER_CVC5_STATUS_UNSAT;
  }
  if (cvc5_result_is_unknown(cvc5_result)) {
    Cvc5UnknownExplanation explanation =
        cvc5_result_get_unknown_explanation(cvc5_result);
    const char* explanation_text =
        cvc5_unknown_explanation_to_string(explanation);
    if (diagnostics != nullptr) {
      *diagnostics = "unknown: " + cstr(explanation_text);
    }
    if (explanation == CVC5_UNKNOWN_EXPLANATION_TIMEOUT) {
      return SOTER_CVC5_STATUS_TIMEOUT;
    }
    return SOTER_CVC5_STATUS_UNKNOWN;
  }
  if (diagnostics != nullptr) {
    *diagnostics = "null or unsupported CVC5 result";
  }
  return SOTER_CVC5_STATUS_ERROR;
}

SoterCvc5Status status_from_output(const std::string& output,
                                   std::string* diagnostics) {
  if (output.find("sat") != std::string::npos
      && output.find("unsat") == std::string::npos) {
    return SOTER_CVC5_STATUS_SAT;
  }
  if (output.find("unsat") != std::string::npos) {
    return SOTER_CVC5_STATUS_UNSAT;
  }
  if (output.find("unknown") != std::string::npos) {
    if (diagnostics != nullptr && diagnostics->empty()) {
      *diagnostics = "unknown";
    }
    return SOTER_CVC5_STATUS_UNKNOWN;
  }
  if (diagnostics != nullptr) {
    *diagnostics = "unable to classify check-sat output: " + output;
  }
  return SOTER_CVC5_STATUS_ERROR;
}

std::string model_string(Cvc5* solver, Cvc5SymbolManager* sm) {
  size_t sort_count = 0;
  size_t term_count = 0;
  const Cvc5Sort* sorts = cvc5_sm_get_declared_sorts(sm, &sort_count);
  const Cvc5Term* terms = cvc5_sm_get_declared_terms(sm, &term_count);
  Cvc5Sort empty_sort = nullptr;
  Cvc5Term empty_term = nullptr;
  if (sorts == nullptr) {
    sorts = &empty_sort;
  }
  if (terms == nullptr) {
    terms = &empty_term;
  }
  return cstr(
      cvc5_get_model(solver, sort_count, sorts, term_count, terms));
}

std::string unsat_core_string(Cvc5* solver) {
  size_t core_count = 0;
  const Cvc5Term* core = cvc5_get_unsat_core(solver, &core_count);
  std::ostringstream out;
  out << "(unsat-core";
  for (size_t i = 0; i < core_count; ++i) {
    out << "\n  " << cstr(cvc5_term_to_string(core[i]));
  }
  out << "\n)";
  return out.str();
}

}  // namespace

const char* soter_cvc5_version(void) {
  static std::string version;

  Cvc5TermManager* tm = cvc5_term_manager_new();
  Cvc5* solver = cvc5_new(tm);
  version = cvc5_get_version(solver);
  cvc5_delete(solver);
  cvc5_term_manager_delete(tm);

  return version.c_str();
}

SoterCvc5SolveResult soter_cvc5_check_sat_smt2(
    const char* smtlib,
    uint64_t timeout_ms,
    bool produce_model,
    bool produce_unsat_core) {
  if (smtlib == nullptr || std::strlen(smtlib) == 0) {
    return result(SOTER_CVC5_STATUS_ERROR, "SMT-LIB input is empty");
  }

  Cvc5TermManager* tm = nullptr;
  Cvc5* solver = nullptr;
  Cvc5InputParser* parser = nullptr;
  SoterCvc5Status status = SOTER_CVC5_STATUS_ERROR;
  std::string diagnostics;
  std::string model;
  std::string unsat_core;

  try {
    tm = cvc5_term_manager_new();
    solver = cvc5_new(tm);

    if (produce_model) {
      cvc5_set_option(solver, "produce-models", "true");
    }
    if (produce_unsat_core) {
      cvc5_set_option(solver, "produce-unsat-cores", "true");
    }
    if (timeout_ms > 0) {
      std::string timeout = std::to_string(timeout_ms);
      cvc5_set_option(solver, "tlimit-per", timeout.c_str());
    }

    parser = cvc5_parser_new(solver, nullptr);
    cvc5_parser_set_str_input(
        parser, CVC5_INPUT_LANGUAGE_SMT_LIB_2_6, smtlib, "soter-smtlib");
    Cvc5SymbolManager* sm = cvc5_parser_get_sm(parser);

    bool saw_check_sat = false;
    while (true) {
      const char* parse_error = nullptr;
      Cvc5Command cmd = cvc5_parser_next_command(parser, &parse_error);
      if (parse_error != nullptr) {
        diagnostics = cstr(parse_error);
        status = SOTER_CVC5_STATUS_ERROR;
        break;
      }
      if (cmd == nullptr) {
        break;
      }

      std::string command_name = cstr(cvc5_cmd_get_name(cmd));
      if (command_name == "check-sat") {
        saw_check_sat = true;
        Cvc5Result check_result = cvc5_check_sat(solver);
        status = status_from_result(check_result, &diagnostics);
        cvc5_result_release(check_result);
      } else {
        std::string output = cstr(cvc5_cmd_invoke(cmd, solver, sm));
        if (command_name == "check-sat-assuming") {
          saw_check_sat = true;
          status = status_from_output(output, &diagnostics);
        }
      }
    }

    if (!saw_check_sat && status != SOTER_CVC5_STATUS_ERROR) {
      diagnostics = "SMT-LIB input did not execute a check-sat command";
      status = SOTER_CVC5_STATUS_ERROR;
    }

    if (status == SOTER_CVC5_STATUS_SAT && produce_model) {
      model = model_string(solver, sm);
    } else if (status == SOTER_CVC5_STATUS_UNSAT && produce_unsat_core) {
      unsat_core = unsat_core_string(solver);
    }
  } catch (const std::exception& err) {
    diagnostics = err.what();
    status = SOTER_CVC5_STATUS_ERROR;
  } catch (...) {
    diagnostics = "unknown CVC5 exception";
    status = SOTER_CVC5_STATUS_ERROR;
  }

  if (parser != nullptr) {
    cvc5_parser_delete(parser);
  }
  if (solver != nullptr) {
    cvc5_delete(solver);
  }
  if (tm != nullptr) {
    cvc5_term_manager_delete(tm);
  }

  return result(status, diagnostics, model, unsat_core);
}

void soter_cvc5_solve_result_delete(SoterCvc5SolveResult* result) {
  if (result == nullptr) {
    return;
  }
  std::free(result->model);
  std::free(result->unsat_core);
  std::free(result->diagnostics);
  result->model = nullptr;
  result->unsat_core = nullptr;
  result->diagnostics = nullptr;
}
