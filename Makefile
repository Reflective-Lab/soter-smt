CVC5_TAG     := cvc5-1.3.3
CVC5_COMMIT  := 8ff882e3e42f046867d2ac2e33e92b3d026144ae
CVC5_SRC     := vendor/cvc5
CVC5_BUILD   := $(CVC5_SRC)/build
CVC5_CONFIGURE_FLAGS ?= --no-poly
JOBS         := $(shell nproc 2>/dev/null || sysctl -n hw.logicalcpu 2>/dev/null || echo 1)

.PHONY: all cvc5 clean distclean

all: cvc5

cvc5: $(CVC5_BUILD)/.soter-$(CVC5_TAG)

$(CVC5_BUILD)/.soter-$(CVC5_TAG): Makefile
	@if [ ! -d $(CVC5_SRC) ]; then \
	  git clone --depth 1 --branch $(CVC5_TAG) \
	    https://github.com/cvc5/cvc5 $(CVC5_SRC); \
	elif [ ! -d $(CVC5_SRC)/.git ]; then \
	  echo "$(CVC5_SRC) exists but is not a git checkout"; \
	  exit 1; \
	fi
	@set -e; \
	current_tag=$$(git -C $(CVC5_SRC) describe --tags --exact-match 2>/dev/null || true); \
		if [ "$$current_tag" != "$(CVC5_TAG)" ]; then \
		  echo "Switching CVC5 from $${current_tag:-unknown} to $(CVC5_TAG)"; \
		  git -C $(CVC5_SRC) fetch --depth 1 origin tag $(CVC5_TAG); \
		  git -C $(CVC5_SRC) switch --detach $(CVC5_TAG); \
		  rm -rf $(CVC5_BUILD); \
		fi; \
		actual_commit=$$(git -C $(CVC5_SRC) rev-parse HEAD); \
		if [ "$$actual_commit" != "$(CVC5_COMMIT)" ]; then \
		  echo "CVC5 $(CVC5_TAG) resolved to $$actual_commit, expected $(CVC5_COMMIT)"; \
		  exit 1; \
		fi
	cd $(CVC5_SRC) && ./configure.sh production \
	  --prefix=$(abspath $(CVC5_BUILD)/install) \
	  --auto-download \
	  $(CVC5_CONFIGURE_FLAGS)
	cmake --build $(CVC5_BUILD) -j$(JOBS)
	cmake --install $(CVC5_BUILD)
	@touch $@

clean:
	rm -rf $(CVC5_BUILD)

distclean:
	rm -rf vendor/
