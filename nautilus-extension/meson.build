nautilus_ext = dependency('libnautilus-extension-4', version: '>= 4.0')
gio_dep = dependency('gio-2.0', version: '>= 2.76')
gio_unix_dep = dependency('gio-unix-2.0', version: '>= 2.76')
glib_dep = dependency('glib-2.0', version: '>= 2.76')

nautilus_extension_dir = nautilus_ext.get_variable(pkgconfig: 'extensiondir')

nautilus_sources = files(
  'src/lnxdrive-extension.c',
  'src/lnxdrive-dbus-client.c',
  'src/lnxdrive-info-provider.c',
  'src/lnxdrive-menu-provider.c',
  'src/lnxdrive-column-provider.c',
)

shared_module(
  'lnxdrive-nautilus',
  nautilus_sources,
  dependencies: [nautilus_ext, gio_dep, gio_unix_dep, glib_dep],
  c_args: [
    '-DGETTEXT_PACKAGE="@0@"'.format(gettext_domain),
    '-DLOCALEDIR="@0@"'.format(localedir),
  ],
  install: true,
  install_dir: nautilus_extension_dir,
)

# Emblem icons — SVG emblems scale natively for HiDPI (FR-029, ref pending issue G2)
emblem_icons = [
  'icons/lnxdrive-synced.svg',
  'icons/lnxdrive-cloud-only.svg',
  'icons/lnxdrive-syncing.svg',
  'icons/lnxdrive-pending.svg',
  'icons/lnxdrive-conflict.svg',
  'icons/lnxdrive-error.svg',
  'icons/lnxdrive-unknown.svg',
]

install_data(
  emblem_icons,
  install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'emblems',
)
