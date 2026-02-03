# LNXDrive GNOME

Integración nativa de LNXDrive para el escritorio GNOME.

## Descripción

Este repositorio proporciona la experiencia de usuario completa para GNOME, incluyendo:

- **Panel de preferencias**: Aplicación GTK4 + libadwaita para configuración
- **Extensión Nautilus**: Overlay icons y menús contextuales
- **Extensión GNOME Shell**: Indicador en la barra de estado
- **Integración GOA**: Soporte para GNOME Online Accounts

## Requisitos

- GNOME 45+
- GTK4 4.12+
- libadwaita 1.4+
- Rust 1.75+
- lnxdrive-daemon en ejecución

## Compilación

```bash
cargo build --release
```

## Instalación

### Desde Flatpak (recomendado)

```bash
flatpak install flathub org.enigmora.LNXDrive
```

### Desde código fuente

```bash
cargo install --path .
```

## Estructura

```
lnxdrive-gnome/
├── src/                  # Código fuente Rust
├── data/
│   ├── icons/           # Iconos de la aplicación
│   └── ui/              # Archivos .ui (Blueprint/XML)
└── extensions/
    ├── nautilus/        # Extensión de Nautilus
    └── gnome-shell/     # Extensión de GNOME Shell
```

## Comunicación con el daemon

La aplicación se comunica con `lnxdrive-daemon` a través de D-Bus usando la librería `lnxdrive-ipc`.

## Licencia

GPL-3.0-or-later
