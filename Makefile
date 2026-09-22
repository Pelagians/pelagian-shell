CARGO ?= cargo
VERSION ?= 0.1.0
REVISION ?= $(shell git rev-parse HEAD 2>/dev/null || printf unknown)
IMAGE ?= pelagian-shell:$(VERSION)
ENGINE ?= docker

.PHONY: check fmt lint test runtime-contract container-build container-smoke

check: fmt lint test runtime-contract

fmt:
	$(CARGO) fmt --all -- --check

lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

test:
	$(CARGO) test --workspace --locked

runtime-contract:
	python3 -m unittest tests.test_reference_runtime tests.test_planner_only_boundary tests.test_consumer_session tests.test_layoutd_supervisor
	python3 -m unittest discover -s tests/consumer-conformance -p 'test_*.py'
	bash -n tests/consumer-conformance/start-shell-stream.sh
	python3 -m py_compile tests/consumer-conformance/verify-shell-session.py

container-build:
	$(ENGINE) build --build-arg VERSION=$(VERSION) --build-arg REVISION=$(REVISION) -f Containerfile -t $(IMAGE) .

container-smoke: container-build
	ENGINE=$(ENGINE) sh tests/container-smoke.sh $(IMAGE)
