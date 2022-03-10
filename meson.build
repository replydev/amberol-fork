# SPDX-FileCopyrightText: 2022  Emmanuele Bassi
# SPDX-License-Identifier: GPL-3.0-or-later

project('amberol', 'rust', 
  version: '0.1.0',
  license: ['GPL-3.0'],
  meson_version: '>= 0.59.0',
  default_options: [ 'warning_level=2', ],
)

i18n = import('i18n')
gnome = import('gnome')

cargo_sources = files(
  'Cargo.toml',
  'Cargo.lock',
)

application_id = 'io.bassi.Amberol'

subdir('data')
subdir('src')
subdir('po')

reuse = find_program('reuse', required: false)
if reuse.found()
  test('license',
    reuse,
    args:['lint'],
    workdir: meson.project_source_root(),
    suite: ['lint'],
  )
endif

gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)
