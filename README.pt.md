<div align="center">
<sub>🇬🇧 <a href="./README.md">English</a> &nbsp;|&nbsp; 🇨🇳 <a href="./README.zh-CN.md">中文</a> &nbsp;|&nbsp; 🇪🇸 <a href="./README.es.md">Español</a> &nbsp;|&nbsp; 🇯🇵 <a href="./README.ja.md">日本語</a> &nbsp;|&nbsp; 🇧🇷 <a href="./README.pt.md">Português</a></sub>
</div>

<br/>

<div align="center">

<img src="logo.png" alt="Perch" width="120">

# Perch

**Um gerenciador de processos e serviços para Windows, leve e de binário único, com web UI integrada.**

Escrito em Rust. Sem dependências de runtime. Um binário, duplo clique e pronto.

</div>

---

## Capturas de tela

> Coloque os arquivos de captura de tela nos caminhos abaixo (ver [Arquivos de captura de tela a fornecer](#arquivos-de-captura-de-tela-a-fornecer)).

### Menu interativo (duplo clique em `perch.exe`)

![menu](docs/screenshots/menu.png)

### Login

![login](docs/screenshots/login.png)

### Painel de processos

![processes](docs/screenshots/processes.png)

### Detalhes do processo — Transmissão de logs ao vivo

![process-detail](docs/screenshots/process-detail.png)

### Tarefas agendadas

![tasks](docs/screenshots/tasks.png)

---

## Por que Perch?

Gerenciar processos em segundo plano e serviços de longa duração no Windows é mais difícil do que deveria ser:

- **Task Scheduler** encerra por padrão processos de longa duração (timeouts de sessão, encerramento da árvore de processos, dependências de sessão de login) — nunca foi projetado para daemons.
- **NSSM / WinSw / Shawl** são um wrapper por serviço. Sem painel, sem visão central, registrar N vezes.
- **PM2** precisa de um runtime de Node.js, seu daemon inativo consome 40–80 MB, e é um cidadão de segunda classe no Windows (inicialização não confiável, gambiarras no registro).
- **Supervisor** nem funciona no Windows.

Perch preenche essa lacuna: **registrar uma vez, gerenciar tudo.** Um único serviço nativo supervisiona todos os seus processos, expõe uma web UI e fica fora do caminho.

## Recursos

- **Binário único** — sem runtime, sem dependências. A web UI é incorporada diretamente no executável.
- **Serviço nativo do Windows** — registra uma vez, inicia com o sistema, sobrevive ao logout, executa em Session 0.
- **Baixo consumo** — compilado em Rust, um único processo supervisor. Alvo de 5–15 MB de memória residente.
- **Web UI integrada** — veja o status, inicie/pare/reinicie processos e acompanhe os logs pelo navegador. Gerencie remotamente, não apenas pelo console.
- **Reinício automático** — processos que falham são reiniciados automaticamente, com backoff configurável.
- **Tarefas agendadas** — agendamento estilo cron para trabalhos periódicos.
- **Rotação diária de logs** — logs da aplicação são escritos em `logs/perch.YYYY-MM-DD.log` ao lado do executável, formato delimitado por pipes.
- **Menu interativo** — duplo clique no exe para instalar/desinstalar/iniciar/parar o serviço e abrir a web UI, com elevação UAC sob demanda.

## Status

> ⚠️ **Desenvolvimento inicial.** Perch está em construção ativa e ainda não está pronto para produção. Interfaces e configuração podem mudar.

## Início rápido

Duplo clique em `perch.exe` e selecione `5) Open Web UI (Abrir Web UI)` no menu — Perch gerará automaticamente um `perch.toml` padrão (com um processo `demo` e o token `change-me`) e iniciará o servidor.

Ou pela linha de comando:

```
# Executar em primeiro plano (Ctrl-C para parar)
perch run

# Registrar Perch como serviço do Windows (precisa de admin)
perch install
perch start

# Abrir a web UI
http://127.0.0.1:9000
```

O token padrão é `change-me` — altere-o em `perch.toml` antes de qualquer uso real.

### Primeira execução: SmartScreen do Windows

O binário distribuído não é assinado digitalmente, então no primeiro uso o SmartScreen do Windows pode exibir **"O Windows protegeu o computador"**. Isso é esperado para ferramentas de código aberto não assinadas. Clique em **Mais informações → Executar assim mesmo** para continuar. (Baixar o `.zip` evita o aviso de download do navegador; o aviso do SmartScreen na primeira execução é outra etapa, inevitável sem um certificado de assinatura de código pago.)

## Segurança

O endpoint de gerenciamento pode iniciar e parar processos arbitrários — ele tem **privilégios muito altos**. Perch vincula-se a `127.0.0.1` por padrão e exige um token de acesso. Não exponha a porta de gerenciamento a redes não confiáveis.

## Comparação

| | Perch | NSSM / WinSw | PM2 | Supervisor |
|---|:---:|:---:|:---:|:---:|
| Binário único | ✅ | ✅ | ❌ (precisa de Node) | ❌ (precisa de Python) |
| Gerencia muitos processos de uma instância | ✅ | ❌ (1 por serviço) | ✅ | ✅ |
| Web UI integrada | ✅ | ❌ | ✅ | ✅ |
| Serviço nativo do Windows | ✅ | ✅ | ⚠️ | ❌ |
| Tarefas agendadas | ✅ | ❌ | ✅ | ⚠️ |
| Rotação diária de logs | ✅ | ❌ | ⚠️ | ⚠️ |
| Baixo consumo de memória | ✅ | ✅ | ❌ | ⚠️ |

## Compilar a partir do código-fonte

```
cargo build --release
```

Requer uma toolchain estável recente de Rust.

## Arquivos de captura de tela a fornecer

Coloque os arquivos de captura de tela em `docs/screenshots/` (crie o diretório primeiro), usando estes nomes exatos:

| Caminho | O que capturar |
|---|---|
| `docs/screenshots/menu.png` | Janela do console após duplo clique em `perch.exe`, mostrando o menu bilíngue (Instalar/Desinstalar/Iniciar/Parar/Abrir Web UI/Sair). |
| `docs/screenshots/login.png` | Página de login de `http://127.0.0.1:9000/` com o logo do Perch e o campo de token. |
| `docs/screenshots/processes.png` | Painel `/processes` com o cartão `demo` mostrando o status Running e a barra de navegação superior com o logo. |
| `docs/screenshots/process-detail.png` | Página `/process-detail?name=demo` com o painel PID/CPU/Mem/Tempo de atividade e o fluxo de logs ao vivo. |
| `docs/screenshots/tasks.png` | Página `/tasks` com pelo menos uma linha de tarefa agendada, barra de navegação superior com o logo. |

Dica: capture com viewport de 1280×800; use PNG; recorte as bordas do navegador. Assim que os arquivos estiverem no lugar, as referências de imagem acima serão renderizadas automaticamente.

## Licença

A definir
