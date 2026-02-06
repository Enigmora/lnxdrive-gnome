project(
  'lnxdrive-gnome',
  'c',
  version: '0.1.0',
  license: 'GPL-3.0-or-later',
  meson_version: '>= 0.62.0',
  default_options: [
    'c_std=c11',
    'warning_level=2',
  ],
)

gnome = import('gnome')
i18n = import('i18n')

# Application and gettext domain
app_id = 'com.enigmora.LNXDrive'
gettext_domain = 'lnxdrive-gnome'

# Standard directories
prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
datadir = prefix / get_option('datadir')
libdir = prefix / get_option('libdir')
localedir = prefix / get_option('localedir')

# Subdirectories (conditional on build options)
if get_option('enable_nautilus')
  subdir('nautilus-extension')
endif

if get_option('enable_shell')
  subdir('shell-extension')
endif

if get_option('enable_preferences')
  subdir('preferences')
endif

subdir('data')
subdir('po')

# Post-install scripts
gnome.post_install(
  gtk_update_icon_cache: true,
  update_desktop_database: true,
  glib_compile_schemas: true,
)
