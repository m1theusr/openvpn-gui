# OpenVPN GUI

Cliente OpenVPN gráfico para Linux (GNOME) desenvolvido em Rust com GTK4 e Libadwaita.

![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)
![GTK4](https://img.shields.io/badge/GTK4-4.12+-blue?logo=gnome)
![License](https://img.shields.io/badge/License-GPL--3.0-green)

## Sobre

O **OpenVPN GUI** é um cliente leve e moderno para gerenciar conexões OpenVPN no Linux. Inspirado no visual do OpenVPN Connect para Windows, oferece uma interface limpa com suporte a múltiplos perfis, monitoramento de tráfego em tempo real e integração com a bandeja do sistema.

## Funcionalidades

- **Importação de perfis `.ovpn`** — importe arquivos de configuração OpenVPN com um clique
- **Autenticação** — suporte a login com usuário e senha, com opção de salvar credenciais
- **Toggle de conexão** — conecte/desconecte com um switch por perfil
- **Monitoramento em tempo real** — gráfico de velocidade (download/upload), bytes transferidos por perfil
- **Painel de status (tray)** — painel rápido acessível pelo ícone na bandeja do sistema
- **Bandeja do sistema** — ícone com menu para abrir o app, painel de status ou sair
- **Interface moderna** — design compacto (360px) inspirado no OpenVPN Connect, tema escuro nativo

## Screenshots

| Janela principal | Painel de status (tray) |
|---|---|
| Toggle + gráfico por perfil | Velocidade, gráfico e lista de perfis |

## Dependências

### Runtime

- `gtk4` >= 4.12
- `libadwaita-1` >= 1.4
- `openvpn` (CLI)
- `dbus` (para bandeja do sistema via StatusNotifierItem)

### Build

- `rust` >= 1.70
- `cargo`
- `pkg-config`
- `gtk4-devel` / `libgtk-4-dev`
- `libadwaita-devel` / `libadwaita-1-dev`
- `glib2-devel` / `libglib2.0-dev`
- `dbus-devel` / `libdbus-1-dev`

## Compilação

```bash
# Instalar dependências (Fedora/Nobara)
sudo dnf install gtk4-devel libadwaita-devel glib2-devel dbus-devel rust cargo pkg-config openvpn

# Instalar dependências (Ubuntu/Debian)
sudo apt install libgtk-4-dev libadwaita-1-dev libglib2.0-dev libdbus-1-dev rustc cargo pkg-config openvpn

# Compilar
cargo build --release

# O binário estará em:
# target/release/openvpn-gui
```

## Executar

```bash
# Modo debug
cargo run

# Modo release (otimizado)
cargo run --release

# Ou executar o binário diretamente
./target/release/openvpn-gui
```

## Estrutura do Projeto

```
openvpn-gui/
├── Cargo.toml              # Dependências e metadados
├── build.rs                # Script de build (GResource)
├── data/
│   └── resources/
│       ├── resources.gresource.xml
│       ├── window.ui       # Template da janela principal
│       └── openvpn-logo.svg
└── src/
    ├── main.rs             # Entry point, tray, polling
    ├── app.rs              # AdwApplication subclass
    ├── window.rs           # Janela principal (perfis, status)
    ├── panel.rs            # Painel de status (tray popup)
    ├── profile/
    │   ├── mod.rs
    │   ├── model.rs        # Modelo VpnProfile
    │   ├── row.rs          # Linha de perfil (toggle, gráfico, stats)
    │   ├── import.rs       # Importação de .ovpn
    │   ├── storage.rs      # Persistência JSON
    │   └── auth.rs         # Diálogos de autenticação
    ├── vpn/
    │   ├── mod.rs
    │   ├── manager.rs      # Conexão/desconexão via openvpn CLI
    │   └── status.rs       # Enum de status
    └── tray/
        ├── mod.rs
        └── indicator.rs    # Bandeja do sistema (ksni)
```

## Armazenamento

- **Perfis:** `~/.config/openvpn-gui/profiles.json`
- **Configs .ovpn:** `~/.config/openvpn-gui/*.ovpn`
- **Ícone:** `~/.local/share/icons/hicolor/scalable/apps/openvpn-gui.svg`
- **Runtime (PIDs, logs, mgmt):** `/tmp/openvpn-gui/`

## Tecnologias

| Tecnologia | Uso |
|---|---|
| **Rust** | Linguagem principal |
| **GTK4** | Framework de UI |
| **Libadwaita** | Componentes GNOME modernos |
| **Cairo** | Renderização de gráficos |
| **ksni** | Integração com bandeja do sistema (SNI) |
| **serde/serde_json** | Serialização de perfis |
| **OpenVPN CLI** | Backend de conexão VPN |

## Licença

Este projeto é licenciado sob a [GPL-3.0](LICENSE).

## Autor

Desenvolvido por [m1theusr](https://github.com/m1theusr).
