#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum SoterCvc5Status {
  SOTER_CVC5_STATUS_SAT = 0,
  SOTER_CVC5_STATUS_UNSAT = 1,
  SOTER_CVC5_STATUS_UNKNOWN = 2,
  SOTER_CVC5_STATUS_TIMEOUT = 3,
  SOTER_CVC5_STATUS_ERROR = 4,
} SoterCvc5Status;

typedef struct SoterCvc5SolveResult {
  SoterCvc5Status status;
  char* model;
  char* unsat_core;
  char* diagnostics;
} SoterCvc5SolveResult;

const char* soter_cvc5_version(void);
SoterCvc5SolveResult soter_cvc5_check_sat_smt2(
    const char* smtlib,
    uint64_t timeout_ms,
    bool produce_model,
    bool produce_unsat_core);
void soter_cvc5_solve_result_delete(SoterCvc5SolveResult* result);

#ifdef __cplusplus
}
#endif
