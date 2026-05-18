.PHONY: dev build test check infra-up infra-stop infra-down infra-clean generate

# Variables
ENVIRONMENT ?= development
PORT ?= 8888

# Starts the development server.
# Uses `cargo watch` for hot reloading, falling back to simple `cargo run` if not installed.
dev:
	@echo "🚀 Iniciando servidor de desenvolvimento Rust..."
	@if command -v cargo-watch >/dev/null 2>&1; then \
		cargo watch -x "run --bin backend-rust"; \
	elif [ -f $(HOME)/.cargo/bin/cargo-watch ]; then \
		$(HOME)/.cargo/bin/cargo-watch -x "run --bin backend-rust"; \
	else \
		echo "⚠️  cargo-watch não instalado no PATH. Executando diretamente..."; \
		cargo run --bin backend-rust; \
	fi

# Builds a release-optimized production binary.
build:
	@echo "📦 Compilando binário de produção otimizado..."
	cargo build --release

# Runs native Rust unit/integration tests.
test:
	@echo "🧪 Executando testes unitários..."
	cargo test

# Performs a static analysis check on the codebase.
check:
	@echo "🔍 Executando verificação estática do código..."
	cargo check

# Runs the CRUD generator. Example: make generate name=Product
generate:
	@echo "⚙️  Executando gerador de CRUD Rust para $(name)..."
	cargo run --bin generator $(name)

# Docker Infrastructure Management (Standard Prefix: infra-)
infra-up:
	@echo "🐳 Subindo infraestrutura local Rust (Postgres & Redis)..."
	docker compose -f docker-compose.infra.yml up -d

infra-stop:
	@echo "🛑 Parando serviços da infraestrutura..."
	docker compose -f docker-compose.infra.yml stop

infra-down:
	@echo "🗑️  Removendo containers da infraestrutura..."
	docker compose -f docker-compose.infra.yml down

infra-clean:
	@echo "🧹 Limpeza completa da infraestrutura (Volumes & Imagens)..."
	docker compose -f docker-compose.infra.yml down -v --rmi all
