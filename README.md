# Advanced Rust Backend API 🦀

Uma arquitetura robusta, segura e ultra-performática construída em **Rust**, utilizando as melhores práticas da linguagem para entregar tipagem estática, máxima concorrência assíncrona, arquitetura modular e aderência perfeita às premissas de LGPD, Auditoria, Observabilidade e RBAC do **Mage Compliance Standard**.

---

## 🚀 Tecnologias Core

O projeto utiliza o estado da arte do ecossistema Rust assíncrono:

- **Runtime Assíncrono:** [Tokio](https://tokio.rs/) (O padrão da indústria para alta concorrência)
- **Framework Web:** [Axum](https://github.com/tokio-rs/axum) (Altamente modular, rápido e baseado na stack robusta de `tower` e `hyper`)
- **ORM:** [Sea-ORM](https://www.sea-ql.org/) (ORM premium assíncrono com tipagem segura baseado em SQLx)
- **Database:** PostgreSQL (Principal) & Redis (Cache, Controle de Sessão e Rate Limit)
- **Documentação:** Swagger (OpenAPI 3.0) via `utoipa` com UI integrada em `/v1/docs`
- **Linter & Formatter:** Clippy & Rustfmt (Garantia de 100% clean-code)
- **Mensageria de E-mail:** Lettre 0.11 (Integração assíncrona robusta via SMTP)

---

## ✨ Principais Funcionalidades

### 🔐 Segurança e Autenticação
- **RBAC Dinâmico e Modular:** Controle de acesso baseado em perfis (Roles) com permissões granulares por feature (`view`, `create`, `delete`, `activate`) encapsulado em uma macro declarativa limpa e intuitiva: `auth_route!`.
- **Gerenciamento de Sessão via Redis:** Rastreamento de tokens JWT em tempo real no cache Redis com suporte a invalidação instantânea no logout ou em atualizações críticas (mudança de status de usuário ou perfil).
- **Rate Limiting Global:** Middleware nativo Axum integrado ao Redis para mitigar abusos e ataques de força bruta, aplicando restrições dinâmicas de chamadas.

### 🏗️ Arquitetura Core (Base Layer)
- **Arquitetura Sem Repositório Boilerplate (DIP):** Chamadas diretas do banco a partir dos Services com Sea-ORM, mantendo o código conciso, ágil e livre de padrões redundantes que poluem o projeto.
- **Response Mappings Elegantes:** Implementação idiomática da trait `From` para converter registros do banco de dados em DTOs de resposta, eliminando mapeamentos manuais repetitivos dos Services.
- **Filtragem Dinâmica:** Módulo `QueryValidator` robusto capaz de validar campos, ordenar dinamicamente, impor limites rígidos de paginação e validar ranges de data de forma automática.
- **Paginação Genérica DRY:** Helper genérico `.paginate()` integrado ao parse de filtros que reduz a lógica de busca e contagem do banco a uma linha simples de código.
- **Soft Delete e LGPD:** Suporte nativo a exclusão lógica (`is_deleted`), combinado com a anonimização automática de dados de usuário em conformidade com as regras da LGPD.

### 📧 Email Infrastructure (DIP)
- **Decoupled Architecture:** Abstração completa através da trait assíncrona `EmailService`, permitindo injeção limpa de drivers.
- **Mock Driver:** `MockEmailService` para simulação visual de e-mails em console durante testes e desenvolvimento.
- **SMTP Driver:** `SmtpEmailService` assíncrono completo que utiliza a biblioteca `lettre` 0.11 com suporte a TLS, credenciais e variáveis de ambiente configuráveis.

### 📄 PDF Service Integration (Streaming Bypass)
- **Zero Memory Footprint:** O backend funciona como um proxy de streaming direto para o microserviço de PDF. O payload gerado em bytes é transmitido instantaneamente ao cliente sem carregar dados em memória ou disco local.
- **Endpoints de Debug:** Rotas GET/POST dedicadas para validar visualmente templates PDF.

### 📊 Real-time Observability (Prometheus & Health Check)
- **Métricas Nativas:** Endpoint `/metrics` exportando dados em tempo real sobre requisições, latências e concorrência para Prometheus.
- **Liveness & Health Check:** Endpoints rápidos de diagnóstico de saúde no caminho `/health` e `/liveness`.

---

## 🛠️ Guia de Desenvolvimento (Fluxo do Generator)

A estrutura de novos CRUDs pode ser criada em segundos usando o nosso gerador automático nativo.

### 🏗️ Geração de Módulos (CRUD)

### 1. Definir a Entidade
Execute uma nova migração Sea-ORM ou SQL para criar a tabela no seu banco PostgreSQL. Garanta que a entidade possua as colunas padrão (`active`, `is_deleted`, `created_at`, `updated_at`) para herdar todos os comportamentos padrão do core.

### 2. Sincronizar o Banco e Rodar Migrações
Execute o banco local via docker e suba as migrações automáticas:
```bash
make infra-up
```
O servidor de desenvolvimento do Rust executa migrações automáticas ao subir.

### 3. Gerar o Módulo
Use a nossa CLI nativa de geração de código para gerar todo o boilerplate (Service, Controller, Schema, Mod, Routes e DTOs) em Rust:

```bash
make generate name=MyNewEntity
```

Este comando irá:
- Criar toda a estrutura em `src/modules/my_new_entity/`.
- Integrar automaticamente os modelos em `src/models/`.
- Estruturar a lógica com injeção automática de filtros e paginação DRY.

---

## ⚙️ Configuração Local

### 🛠️ Instalação de Pré-requisitos
Antes de compilar, instale os cabeçalhos de desenvolvimento do PostgreSQL e OpenSSL em seu sistema Linux:
```bash
sudo apt update
sudo apt install -y build-essential libssl-dev pkg-config libpq-dev
```

### Variáveis de Ambiente
Copie o arquivo `.env.example` para `.env` e configure suas variáveis locais:
```bash
cp .env.example .env
```

### Gerenciamento da Infraestrutura (Docker)
```bash
make infra-up       # Sobe Postgres e Redis em segundo plano
make infra-stop     # Pausa os containers de infraestrutura
make infra-down     # Remove os containers locais de infraestrutura
make infra-clean    # Remove containers, volumes persistidos e imagens locais
```

### Executando o Servidor de Desenvolvimento
```bash
make dev            # Inicia o servidor com hot-reload (cargo watch)
```

---

## 🛡️ Qualidade de Código & Automação Git

Mantemos um padrão de elite absoluto de integridade e limpeza de código:

### Pre-commit Hooks Nativos (Zero Dependencies)
Em vez de sobrecarregar o repositório Rust com ferramentas de ecossistemas externos (NodeJS/Husky), criamos um **Git Pre-commit Hook Nativo**. Para registrá-lo em seu ambiente local, execute uma única vez:

```bash
make init-hooks
```

Este hook irá interceptar seus commits locais e garantir:
1. **`cargo fmt`:** O código deve estar 100% formatado segundo as regras da linguagem.
2. **`cargo clippy`:** Zero warnings permitidas! O commit falhará se houver qualquer desvio de lint apontado pelo compilador.
3. **Detector de Comentários Legados:** Proíbe o commit de restos de códigos comentados (ex: `// let x = 1;`).
4. **Detector de Doc-Comments Vazios:** Bloqueia commits que contenham blocos `///` vazios ou sem explicação descritiva.

---

## 📖 API Documentation & Observability

A documentação interativa e os endpoints integrados ficam disponíveis nas seguintes portas padrão:

- **Swagger UI (OpenAPI 3.0):** `http://localhost:8888/v1/docs`
- **Health Check:** `http://localhost:8888/health`
- **Prometheus Metrics:** `http://localhost:8888/metrics`
- **Liveness Probe:** `http://localhost:8888/liveness`
- **PDF Debug Template (GET/POST):** `http://localhost:8888/v1/debug/pdf`

---

## 🗺️ Roadmap de Features Pendentes (Paridade com Node.js)

Para atingir a paridade total de recursos com a versão avançada em Node.js, os seguintes itens devem ser implementados na stack Rust:

- [ ] **📧 Mensageria (RabbitMQ Integration):**
  - Integração condicional baseada na variável `.env` `MESSAGING_ENABLED=true`.
  - Abstração de um `MessagingProvider` genérico em Rust para publicação e consumo assíncrono de eventos no RabbitMQ.
- [ ] **📁 Cloud Storage Providers (Multi-Provider CLI):**
  - Drivers para **AWS S3**, **Google Cloud Storage (GCS)** e **Azure Blob Storage**.
  - CLI geradora de driver de armazenamento para facilitar a instalação de provedores de nuvem sob demanda com um único comando.
- [ ] **🎛️ Observabilidade Completa (Grafana & Dashboard local):**
  - Configuração do Prometheus e Grafana local com volumes Docker persistidos.
  - Painéis de Grafana prontos para exibição de RPS, latência, códigos de status de rota Axum e métricas de consumo de CPU/Memória do processo.
- [ ] **🖥️ Audit Explorer UI:**
  - Interface administrativa para visualização direta e amigável dos logs de auditoria e das ocorrências de erro capturadas na base de dados.
- [ ] **⚙️ CI/CD Workflow (GitHub Actions):**
  - Automação da esteira de integração contínua (CI) rodando validação estética (`cargo fmt`), análises estáticas rígidas (`cargo clippy`), build completo da aplicação e execução automatizada da suíte de testes a cada Push ou Pull Request.


