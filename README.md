# Mage Backend Boilerplate — Rust Edition 🦀

Este é o boilerplate de referência para o Mage CLI reescrito em **Rust**, utilizando as melhores práticas da linguagem para entregar tipagem estática, máxima performance, arquitetura modular e aderência perfeita às premissas de LGPD, Auditoria e RBAC do **Mage CLI Standard**.

---

## 🛠️ Pré-requisitos do Sistema

Antes de começar o desenvolvimento ou compilar o projeto em sua máquina local Linux (Ubuntu/Debian ou similar), você precisa instalar os pacotes essenciais do sistema para compilar dependências como drivers do PostgreSQL e OpenSSL:

```bash
sudo apt update
sudo apt install -y build-essential libssl-dev pkg-config libpq-dev
```

*   **`libssl-dev`** e **`pkg-config`**: Necessários para que o Rust compile conexões SSL/TLS seguras com o banco e servidores externos.
*   **`libpq-dev`**: Driver nativo do PostgreSQL necessário para compilação do SeaORM / SQLx com suporte a C-bindings de alta performance.

Além disso, verifique se o **Docker** e o **Docker Compose** estão ativos em seu sistema.

---

## 🚀 Como Iniciar

1.  **Subir a Infraestrutura (Postgres & Redis):**
    ```bash
    make infra-up
    ```

2.  **Iniciar o Servidor em Modo de Desenvolvimento (Hot Reload):**
    ```bash
    make dev
    ```

3.  **Executar Auditoria / E2E com o Compliance Suite:**
    Navegue até a pasta do `mage-backend-compliance` e rode:
    ```bash
    make test
    ```
