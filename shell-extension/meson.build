shell_ext_uuid = 'lnxdrive-indicator@enigmora.com'
shell_ext_dir = datadir / 'gnome-shell' / 'extensions' / shell_ext_uuid

shell_ext_sources = files(
  shell_ext_uuid / 'extension.js',
  shell_ext_uuid / 'metadata.json',
  shell_ext_uuid / 'prefs.js',
  shell_ext_uuid / 'stylesheet.css',
  shell_ext_uuid / 'dbus.js',
  shell_ext_uuid / 'indicator.js',
  shell_ext_uuid / 'menuItems.js',
)

install_data(
  shell_ext_sources,
  install_dir: shell_ext_dir,
)
