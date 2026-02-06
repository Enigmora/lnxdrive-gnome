cargo = find_program('cargo')
cp = find_program('cp')

cargo_build_target = custom_target(
  'lnxdrive-preferences',
  output: 'lnxdrive-preferences',
  command: [
    'sh', '-c',
    ' '.join([
      cargo.full_path(), 'build',
      '--manifest-path', meson.current_source_dir() / 'Cargo.toml',
      '--release',
      '--target-dir', meson.current_build_dir() / 'cargo-target',
      '&&',
      cp.full_path(),
      meson.current_build_dir() / 'cargo-target' / 'release' / 'lnxdrive-preferences',
      '@OUTPUT@',
    ]),
  ],
  env: {
    'LOCALEDIR': localedir,
  },
  console: true,
  install: true,
  install_dir: bindir,
)

# Desktop file
i18n.merge_file(
  input: 'data' / 'com.enigmora.LNXDrive.Preferences.desktop.in',
  output: 'com.enigmora.LNXDrive.Preferences.desktop',
  type: 'desktop',
  po_dir: meson.project_source_root() / 'po',
  install: true,
  install_dir: datadir / 'applications',
)

# AppStream metainfo
i18n.merge_file(
  input: 'data' / 'com.enigmora.LNXDrive.Preferences.metainfo.xml.in',
  output: 'com.enigmora.LNXDrive.Preferences.metainfo.xml',
  type: 'xml',
  po_dir: meson.project_source_root() / 'po',
  install: true,
  install_dir: datadir / 'metainfo',
)

# GSettings schema
install_data(
  'data' / 'com.enigmora.LNXDrive.Preferences.gschema.xml',
  install_dir: datadir / 'glib-2.0' / 'schemas',
)
