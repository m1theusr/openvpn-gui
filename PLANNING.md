# OpenVPN GUI — Linux GNOME Client

## Visão Geral

App nativo GNOME para gerenciar conexões OpenVPN com suporte a:
- Importar arquivos `.ovpn`
- Conectar / Desconectar perfis VPN
- Minimizar para bandeja do sistema (tray) ao fechar
- Rodar em segundo plano com ícone indicador
- Encerrar processo completamente via menu

---

## Stack Técnica

| Camada | Tecnologia | Versão mínima |
|---|---|---|
| Linguagem | **Rust** | 1.75+ |
| Toolkit UI | **GTK4** via `gtk4-rs` | 0.9.x |
| Design System | **libadwaita** via `libadwaita-rs` | 0.7.x |
| OpenVPN Backend | **openvpn-rs** (D-Bus bindings p/ OpenVPN 3) | 0.1.x |
| D-Bus (fallback) | **zbus** | 4.x |
| System Tray | **ksni** (StatusNotifierItem / KDE spec) | 0.3.x |
| Build System | **Cargo** + **Meson** (opcional, p/ instalação GNOME) | — |
| Empacotamento | **Flatpak** (distribuição) | — |

### Notas sobre dependências do sistema

```bash
# Fedora / RHEL
sudo dnf install gtk4-devel libadwaita-devel openvpn-client

# Ubuntu / Debian
sudo apt install libgtk-4-dev libadwaita-1-dev openvpn

# Arch
sudo pacman -S gtk4 libadwaita openvpn
```

---

## Arquitetura

```
┌──────────────────────────────────────────────┐
│                   GUI Layer                   │
│         GTK4 + libadwaita (Adwaita)          │
│                                              │
│  ┌─────────┐  ┌──────────┐  ┌────────────┐  │
│  │ Profile  │  │ Connect  │  │  Settings  │  │
│  │  List    │  │  Panel   │  │   Window   │  │
│  └────┬─────┘  └────┬─────┘  └────────────┘  │
│       │              │                        │
├───────┴──────────────┴────────────────────────┤
│              Service Layer (Rust)              │
│                                               │
│  ┌──────────────┐  ┌───────────────────────┐  │
│  │ VPN Manager  │  │  Profile Store (.ovpn) │  │
│  │ (connect/    │  │  (import, parse,       │  │
│  │  disconnect) │  │   persist configs)     │  │
│  └──────┬───────┘  └───────────────────────┘  │
│         │                                     │
├─────────┴─────────────────────────────────────┤
│              System Integration               │
│                                               │
│  ┌──────────────┐  ┌───────────────────────┐  │
│  │ D-Bus Client │  │  Tray Icon (ksni)     │  │
│  │ (openvpn-rs │  │  StatusNotifierItem   │  │
│  │  / zbus)     │  │                       │  │
│  └──────┬───────┘  └───────────────────────┘  │
│         │                                     │
├─────────┴─────────────────────────────────────┤
│         openvpn-linux daemon (D-Bus)         │
│    net.openvpn.v3.configuration               │
│    net.openvpn.v3.sessions                    │
└───────────────────────────────────────────────┘
```

### Fluxo de dados

1. **Importar .ovpn** → Lê arquivo → Envia config via D-Bus (`net.openvpn.v3.configuration`) → Daemon persiste
2. **Conectar** → Cria sessão via D-Bus (`net.openvpn.v3.sessions`) → Monitora status via signals D-Bus
3. **Desconectar** → Encerra sessão via D-Bus
4. **Fechar janela** → Dialog: "Minimizar para bandeja" ou "Encerrar" → Se minimizar: esconde janela, mantém tray icon ativo

---

## Funcionalidades — Fases

### Fase 1 — MVP (Core)

- [ ] **Scaffold do projeto** — Cargo workspace, dependências, build básico
- [ ] **Janela principal** — `AdwApplicationWindow` com `AdwHeaderBar`
- [ ] **Importar .ovpn** — File chooser dialog, parse e armazenamento
- [ ] **Lista de perfis** — `AdwActionRow` listando perfis importados
- [ ] **Conectar / Desconectar** — Comunicação D-Bus com openvpn daemon
- [ ] **Indicador de status** — Badge/ícone mostrando se está conectado
- [ ] **Notificações** — `GNotification` ao conectar/desconectar

### Fase 2 — Tray & Background

- [ ] **Comportamento de fechar** — Interceptar `close-request`, mostrar `AdwAlertDialog`:
  - "Minimizar para bandeja" → esconde janela, mantém app vivo
  - "Encerrar" → `app.quit()`
  - Checkbox "Lembrar minha escolha"
- [ ] **System Tray Icon** — Via `ksni` (StatusNotifierItem):
  - Ícone indica status: desconectado (cinza), conectado (verde), conectando (amarelo)
  - Menu de contexto: Conectar/Desconectar, Abrir janela, Encerrar
- [ ] **Background running** — `GApplication` com `hold()` para manter processo vivo sem janela

### Fase 3 — UX Polish

- [ ] **Preferências** — `AdwPreferencesWindow`:
  - Autoconnect ao iniciar
  - Iniciar minimizado
  - Comportamento padrão ao fechar
  - Tema (seguir sistema / claro / escuro)
- [ ] **Logs de conexão** — Visualizar logs do tunnel em tempo real
- [ ] **Múltiplas conexões** — Suporte a conectar em mais de um perfil
- [ ] **Editar perfis** — Renomear, deletar, editar configs
- [ ] **Autenticação** — Dialog de credenciais quando perfil requer user/pass

### Fase 4 — Distribuição

- [ ] **Desktop Entry** — `.desktop` file para aparecer no GNOME Activities
- [ ] **Ícone do app** — SVG seguindo guidelines GNOME (symbolic + full color)
- [ ] **Flatpak manifest** — `com.github.user.OpenVPNGUI.yml`
- [ ] **Metainfo / AppStream** — Para aparecer no GNOME Software
- [ ] **CI/CD** — GitHub Actions para build e release Flatpak

---

## Estrutura do Projeto

```
openvpn-gui/
├── Cargo.toml
├── Cargo.lock
├── meson.build                    # Build system (para instalação GNOME)
├── data/
│   ├── com.github.user.OpenVPNGUI.desktop.in
│   ├── com.github.user.OpenVPNGUI.metainfo.xml
│   ├── com.github.user.OpenVPNGUI.gschema.xml   # GSettings schema
│   ├── icons/
│   │   ├── scalable/
│   │   │   └── apps/
│   │   │       └── com.github.user.OpenVPNGUI.svg
│   │   └── symbolic/
│   │       └── apps/
│   │           └── com.github.user.OpenVPNGUI-symbolic.svg
│   └── resources/
│       ├── resources.gresource.xml
│       ├── window.ui              # UI definition (Blueprint/XML)
│       └── style.css
├── src/
│   ├── main.rs                    # Entry point, AdwApplication setup
│   ├── app.rs                     # Application struct e setup
│   ├── window.rs                  # MainWindow (AdwApplicationWindow)
│   ├── profile/
│   │   ├── mod.rs
│   │   ├── model.rs               # VpnProfile struct
│   │   ├── row.rs                 # ProfileRow widget (AdwActionRow)
│   │   └── import.rs              # .ovpn file parser/importer
│   ├── vpn/
│   │   ├── mod.rs
│   │   ├── manager.rs             # VPN connection manager (D-Bus)
│   │   ├── session.rs             # Session state machine
│   │   └── status.rs              # ConnectionStatus enum
│   ├── tray/
│   │   ├── mod.rs
│   │   └── indicator.rs           # ksni tray icon implementation
│   ├── dialogs/
│   │   ├── mod.rs
│   │   ├── close_dialog.rs        # Minimizar vs Encerrar
│   │   └── auth_dialog.rs         # Credenciais
│   └── settings.rs                # GSettings wrapper
└── tests/
    ├── profile_import_test.rs
    └── vpn_manager_test.rs
```

---

## Detalhes Técnicos

### 1. Comunicação D-Bus com OpenVPN 3

O `openvpn-linux` expõe 3 serviços principais via D-Bus:

| Serviço D-Bus | Função |
|---|---|
| `net.openvpn.v3.configuration` | Importar/gerenciar perfis .ovpn |
| `net.openvpn.v3.sessions` | Criar/controlar sessões VPN |
| `net.openvpn.v3.log` | Receber logs em tempo real |

**Crate `openvpn-rs`** encapsula essas chamadas. Se insuficiente, usar `zbus` diretamente:

```rust
use zbus::Connection;

let connection = Connection::system().await?;
let proxy = connection.object("net.openvpn.v3.sessions", "/net/openvpn/v3/sessions")?;
```

### 2. System Tray via `ksni`

```rust
use ksni::{Tray, TrayMethods};

struct VpnTray {
    connected: bool,
}

impl Tray for VpnTray {
    fn id(&self) -> String { "openvpn-gui".into() }
    fn title(&self) -> String { "OpenVPN GUI".into() }
    fn icon_name(&self) -> String {
        if self.connected { "network-vpn".into() }
        else { "network-vpn-disconnected".into() }
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            ksni::menu::StandardItem { label: "Abrir".into(), activate: Box::new(|_| { /* show window */ }), ..Default::default() }.into(),
            ksni::menu::StandardItem { label: "Encerrar".into(), activate: Box::new(|_| { std::process::exit(0); }), ..Default::default() }.into(),
        ]
    }
}
```

> **Nota GNOME**: StatusNotifierItem não é suportado nativamente no GNOME Shell. O usuário precisa da extensão [AppIndicator Support](https://extensions.gnome.org/extension/615/appindicator-support/). Isso é padrão em Ubuntu (vem instalado). Para GNOME stock, documentar a necessidade da extensão.

### 3. Comportamento de Fechar Janela

```rust
window.connect_close_request(move |win| {
    if settings.get_boolean("minimize-on-close") {
        win.set_visible(false);
        app.hold(); // mantém app vivo
        return glib::Propagation::Stop;
    }
    // mostrar AdwAlertDialog
    let dialog = adw::AlertDialog::builder()
        .heading("Fechar aplicação")
        .body("O que deseja fazer?")
        .build();
    dialog.add_response("minimize", "Minimizar para bandeja");
    dialog.add_response("quit", "Encerrar");
    dialog.set_default_response(Some("minimize"));
    dialog.set_close_response("minimize");
    dialog.connect_response(None, move |_, response| {
        match response {
            "minimize" => { win.set_visible(false); app.hold(); }
            "quit" => { app.quit(); }
            _ => {}
        }
    });
    dialog.present(Some(win));
    glib::Propagation::Stop
});
```

### 4. Re-abrir janela do tray / segunda instância

Usando `GApplication` (single-instance por padrão), quando o usuário clica no ícone do Activities ou executa o binário novamente:

```rust
app.connect_activate(move |app| {
    if let Some(window) = app.active_window() {
        window.set_visible(true);
        window.present();
    } else {
        let window = MainWindow::new(app);
        window.present();
    }
});
```

---

## Dependências — Cargo.toml

```toml
[package]
name = "openvpn-gui"
version = "0.1.0"
edition = "2021"

[dependencies]
gtk = { package = "gtk4", version = "0.9", features = ["v4_12"] }
adw = { package = "libadwaita", version = "0.7", features = ["v1_4"] }
glib = { package = "glib", version = "0.20" }
gio = { package = "gio", version = "0.20" }
zbus = { version = "4", features = ["tokio"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
ksni = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
log = "0.4"
env_logger = "0.11"

# openvpn-rs se for suficiente, senão usar zbus direto
# openvpn = "0.1"

[build-dependencies]
glib-build-tools = "0.20"
```

---

## Requisitos do Sistema

- **Linux** com GNOME 44+ (GTK 4.12+, libadwaita 1.4+)
- **openvpn-linux** instalado e rodando (`openvpn-service-client`, `openvpn-service-configmgr`, `openvpn-service-sessionmgr`)
- **Extensão AppIndicator** (para ícone no tray em GNOME Shell puro)
- **Rust toolchain** 1.75+

---

## Referências

- [gtk4-rs Book](https://gtk-rs.org/gtk4-rs/stable/latest/book/)
- [libadwaita-rs Docs](https://world.pages.gitlab.gnome.org/Rust/libadwaita-rs/stable/latest/docs/libadwaita/)
- [openvpn-rs](https://github.com/zaxbux/openvpn-rs)
- [openvpn-linux D-Bus API](https://github.com/OpenVPN/openvpn-linux)
- [ksni (StatusNotifierItem)](https://github.com/iovxw/ksni)
- [GNOME HIG](https://developer.gnome.org/hig/)
- [Flatpak Rust Template](https://github.com/bilelmoussaoui/gtk-rust-template)
