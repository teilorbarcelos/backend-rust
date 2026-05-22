# Advanced Rust Backend API 🦀

Uma arquitetura robusta, segura e ultra-performática construída em **Rust**, utilizando as melhores práticas da linguagem para entregar tipagem estática, máxima concorrência assíncrona, arquitetura modular e aderência perfeita às premissas de LGPD, Auditoria, Observabilidade e RBAC do **Mage Compliance Standard**.

---

## 🚀 Tecnologias Core

O projeto utiliza o estado da arte do ecossistema Rust assíncrono:

- **Runtime Assíncrono:** [Tokio](https://tokio.rs/) (O padrão da indústria para alta concorrência)
- **Framework Web:** [Axum](https://github.com/tokio-rs/axum) (Altamente modular, rápido e baseado na stack robusta de `tower` e `hyper`)
- **ORM:** [Sea-ORM](https://www.sea-ql.org/) (ORM premium assíncrono com tipagem segura baseado em SQLx)
- **Database:** PostgreSQL (Principal) & Redis (Cache, Controle de Sessão e Rate Limit)
- **Mensageria (RabbitMQ):** [lapin](https://github.com/CleverCloud/lapin) (Cliente AMQP 0.9.1 puro em Rust assíncrono)
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

### 💬 Mensageria & Integração com RabbitMQ
- **MessagingProvider Abstraído:** Gerenciador global de conexão AMQP com RabbitMQ integrado opcionalmente via `.env` (`MESSAGING_ENABLED=true`).
- **Publicação e Consumo Resilientes:** Suporta reconexão automática, publicação de mensagens JSON tipadas e tratamento resiliente de erros/rejeição com `nack`.

### 📄 PDF Service Integration (Streaming Bypass)
- **Zero Memory Footprint:** O backend funciona como um proxy de streaming direto para o microserviço de PDF. O payload gerado em bytes é transmitido instantaneamente ao cliente sem carregar dados em memória ou disco local.
- **Endpoints de Debug:** Rotas GET/POST dedicadas para validar visualmente templates PDF.

### 🖥️ Audit Explorer UI
- Interface administrativa construída diretamente no backend que permite consultar, auditar e inspecionar logs de auditoria e ocorrências de erros registradas no banco de dados.

### 📊 Real-time Observability (Prometheus & Grafana)
- **Métricas Nativas:** Endpoint `/metrics` exportando dados em tempo real sobre requisições, latências e concorrência para Prometheus.
- **Painéis de Grafana Prontos:** Grafana local pré-configurado via Docker Compose para visualização visual de CPU, memória, RPS e taxas de status HTTP das rotas Axum.
- **Liveness & Health Check:** Endpoints rápidos de diagnóstico de saúde no caminho `/health` e `/liveness`.

---

## 🛠️ Gerador de Módulos (CLI CRUD Generator) ⚙️

Como o Rust possui uma verbosidade natural devido à sua forte tipagem estática e segurança em tempo de compilação, adicionamos uma ferramenta de CLI interativa para automatizar todo o processo de criação de novos recursos. 

Com um único comando, o gerador automatiza a criação do CRUD completo, a migration correspondente, a documentação Swagger OpenAPI, as permissões RBAC no banco de dados, e a **suíte completa de testes de integração**.

### 🎮 Como utilizar

#### Método 1: Modo Interativo (Recomendado)
Basta digitar o seguinte comando no terminal:
```bash
make generate
```
Se nenhum argumento for fornecido, a CLI iniciará o assistente interativo por prompts com seleção por setas e Enter:

1. **Nome da Entidade:** Digite em PascalCase (ex: `ProductCategory`).
2. **Definição de Campos:** Digite o nome do campo. Em seguida, selecione o tipo e o nível de obrigatoriedade usando as setas:
   - **Tipos disponíveis:** `string` (VARCHAR(255)), `text` (TEXT), `int` (INTEGER), `bool` (BOOLEAN), `decimal` (NUMERIC(10,2)), `float` (DOUBLE PRECISION), `date` (TIMESTAMP WITH TIME ZONE).
   - **Obrigatoriedade:** `Nullable (opcional)` (padrão) ou `Not Null (obrigatório)`.
3. **Registro no RBAC:** Escolha se deseja registrar a feature no sistema de controle de acesso (RBAC). Se sim, informe o ID, nome e descrição da feature.

#### Método 2: Modo Direct CLI (Passagem de Parâmetros)
Você também pode rodar o comando fornecendo os argumentos diretamente no terminal:
```bash
make generate name=Category fields="name:string:notnull description:text active:bool"
```
*Formato:* `campo:tipo` (opcional/nullable por padrão) ou `campo:tipo:notnull` (obrigatório).

---

### 📂 Arquivos Gerados Automaticamente
Ao rodar o gerador para a entidade `Category`, ele criará e registrará a seguinte estrutura de arquivos:

* 📄 **`src/models/category.rs`** - Entidade de banco mapeada via Sea-ORM.
* 📄 **`src/modules/category/schemas.rs`** - DTOs de entrada e saída (CreateRequest, UpdateRequest, Response).
* 📄 **`src/modules/category/service.rs`** - Regras de negócio, paginação DRY e filtragem dinâmica.
* 📄 **`src/modules/category/controller.rs`** - Handlers Axum mapeando requisições e OpenAPI Docs.
* 📄 **`src/modules/category/routes.rs`** - Definição de rotas HTTP protegidas por RBAC.
* 📄 **`src/modules/category/mod.rs`** - Arquivo centralizador do módulo.
* 📄 **`src/migration/mYYYYMMDD_HHMMSS_create_category_table.rs`** - Script de migration SQL para o banco.
* 📄 **`tests/compliance/t17_category.rs`** - Arquivo contendo todos os cenários de testes de integração do CRUD.
* 📝 **`src/models/mod.rs`** - Auto-registro do model.
* 📝 **`src/modules/mod.rs`** - Auto-registro do módulo Axum.
* 📝 **`src/migration/mod.rs`** - Auto-registro do script de migração.
* 📝 **`src/modules/observability.rs`** - Integração automática aos Swagger OpenAPI Docs.
* 📝 **`src/infra/bootstrap.rs`** - Auto-registro da nova feature nos perfis RBAC do banco.
* 📝 **`tests/compliance/mod.rs` & `tests/integration_tests.rs`** - Inclusão da suite de testes de conformidade.

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

### Executando Testes e Cobertura
A suíte de testes de integração conta com um sistema de **limpeza inteligente** que detecta migrações órfãs/removidas de execuções anteriores do gerador de CRUD e limpa a tabela `seaql_migrations` e tabelas deletadas de forma limpa e automática, eliminando problemas de concorrência com testes unitários em paralelo.

```bash
cargo test          # Roda todos os testes (unitários e integração em paralelo)
make coverage       # Gera relatório detalhado de cobertura (cargo tarpaulin)
```

---

## 🛡️ Qualidade de Código & Automação Git

Mantemos um padrão de elite absoluto de integridade e limpeza de código:

### Pre-commit Hooks Nativos (Zero Dependencies)
Para registrar o hook de commit nativo no repositório local, execute uma única vez:

```bash
make init-hooks
```

Este hook interceptará os commits locais e garantirá:
1. **`cargo fmt`:** O código deve estar 100% formatado segundo as regras da linguagem.
2. **`cargo clippy`:** Zero warnings permitidas!
3. **Detector de Comentários Legados:** Proíbe o commit de restos de códigos comentados (ex: `// let x = 1;`).
4. **Doc-Comments:** Evita documentações de código vazias ou incompletas.

---

## 📖 API Documentation & Observability

Portas e URLs padrão dos serviços locais:

- **Swagger UI (OpenAPI 3.0):** `http://localhost:8888/v1/docs`
- **Health Check:** `http://localhost:8888/health`
- **Prometheus Metrics:** `http://localhost:8888/metrics`
- **Liveness Probe:** `http://localhost:8888/liveness`
- **PDF Debug Template (GET/POST):** `http://localhost:8888/v1/debug/pdf`
- **Audit Explorer UI:** `http://localhost:8888/v1/audit/explore`

---

## 🗺️ Roadmap de Features Pendentes (Paridade com Node.js)

Para atingir a paridade total de recursos com a versão avançada em Node.js:

- [X] **📧 Mensageria (RabbitMQ Integration):**
  - Integração condicional baseada na variável `.env` `MESSAGING_ENABLED=true`.
  - Abstração de um `MessagingProvider` genérico em Rust para publicação e consumo assíncrono de eventos no RabbitMQ.
- [ ] **📁 Cloud Storage Providers (Multi-Provider CLI):**
  - Drivers para **AWS S3**, **Google Cloud Storage (GCS)** e **Azure Blob Storage**.
  - CLI geradora de driver de armazenamento para facilitar a instalação de provedores de nuvem sob demanda com um único comando.
- [X] **🎛️ Observabilidade Completa (Grafana & Dashboard local):**
  - Configuração do Prometheus e Grafana local com volumes Docker persistidos.
  - Painéis de Grafana prontos para exibição de RPS, latência, códigos de status de rota Axum e métricas de consumo de CPU/Memória do processo.
- [X] **🖥️ Audit Explorer UI:**
  - Interface administrativa para visualização direta e amigável dos logs de auditoria e das ocorrências de erro capturadas na base de dados.
- [ ] **⚙️ CI/CD Workflow (GitHub Actions):**
  - Automação da esteira de integração contínua (CI) rodando validação estética (`cargo fmt`), análises estáticas rígidas (`cargo clippy`), build completo da aplicação e execução automatizada da suíte de testes a cada Push ou Pull Request.
- [X] **🏗️ Melhorias no Gerador de Módulos (CLI Generator):**
  - O gerador atual já cria Model, Schemas, Service, Controller, Rotas, OpenAPI docs, seedings e testes de integração com o banco automaticamente, e limpa registros de migrações stale para testes paralelos.