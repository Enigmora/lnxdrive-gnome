# Feature Specification: GNOME Desktop Integration

**Feature Branch**: `001-gnome-integration`
**Created**: 2026-02-05
**Status**: Draft
**Input**: User description: "Fase 3 - Integración GNOME: DBus service completo, extensión Nautilus (overlay icons + menú), panel de preferencias GTK4, GNOME Shell extension para status, integración con GNOME Online Accounts."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Ver estado de archivos sincronizados en Nautilus (Priority: P1)

Un usuario de GNOME abre Nautilus y navega a su carpeta de LNXDrive. Inmediatamente ve iconos superpuestos (overlay icons) sobre cada archivo que indican visualmente su estado: sincronizado, solo en la nube, sincronizando, pendiente, en conflicto o con error. El usuario puede distinguir de un vistazo qué archivos están disponibles localmente y cuáles necesitan descargarse. Además, puede ver columnas adicionales con el estado de sincronización y la fecha de última sincronización.

**Why this priority**: Esta es la funcionalidad más fundamental de la integración GNOME. Sin visibilidad del estado de los archivos, el usuario no tiene forma de entender qué está pasando con su sincronización. Es el cimiento sobre el cual se construyen todas las demás interacciones.

**Independent Test**: Puede probarse completamente navegando a la carpeta de LNXDrive en Nautilus y verificando que cada archivo muestra el overlay icon correcto según su estado real de sincronización. Entrega valor inmediato al dar visibilidad total del estado de archivos.

**Acceptance Scenarios**:

1. **Given** un archivo está sincronizado y disponible localmente, **When** el usuario lo ve en Nautilus, **Then** muestra un overlay icon de "sincronizado" (checkmark verde).
2. **Given** un archivo es un placeholder (solo en la nube), **When** el usuario lo ve en Nautilus, **Then** muestra un overlay icon de "en la nube" que lo distingue visualmente de un archivo local.
3. **Given** un archivo se está descargando actualmente, **When** el usuario lo ve en Nautilus, **Then** muestra un overlay icon de "sincronizando" que indica progreso.
4. **Given** un archivo tiene un conflicto de sincronización, **When** el usuario lo ve en Nautilus, **Then** muestra un overlay icon de "conflicto" que alerta visualmente al usuario.
5. **Given** un archivo tuvo un error de sincronización, **When** el usuario lo ve en Nautilus, **Then** muestra un overlay icon de "error" distinguible del resto.
6. **Given** el usuario activa la columna "Estado LNXDrive" en Nautilus, **When** navega por la carpeta sincronizada, **Then** ve el estado textual y la fecha de última sincronización de cada archivo.

---

### User Story 2 - Acciones rápidas desde el menú contextual de Nautilus (Priority: P1)

Un usuario hace clic derecho sobre un archivo o carpeta en su directorio de LNXDrive dentro de Nautilus. Aparece un submenú "LNXDrive" con acciones contextuales relevantes según el estado del archivo: "Mantener disponible offline" para archivos en la nube, "Liberar espacio" para archivos descargados, y "Sincronizar ahora" para forzar una sincronización inmediata. Las opciones se adaptan al estado actual del archivo seleccionado.

**Why this priority**: Las acciones contextuales son igual de críticas que la visibilidad de estado. Permiten al usuario interactuar con la sincronización directamente desde su flujo natural de trabajo en el gestor de archivos, sin necesidad de abrir una aplicación separada.

**Independent Test**: Puede probarse haciendo clic derecho sobre archivos en diferentes estados y verificando que las opciones del menú contextual son las correctas para cada estado, y que ejecutarlas produce el resultado esperado.

**Acceptance Scenarios**:

1. **Given** un archivo es placeholder (solo en la nube), **When** el usuario hace clic derecho y selecciona "Mantener disponible offline", **Then** el archivo se descarga y queda disponible localmente de forma permanente (pinned).
2. **Given** un archivo está disponible localmente y pinned, **When** el usuario hace clic derecho y selecciona "Liberar espacio", **Then** el archivo se convierte en placeholder liberando el espacio en disco.
3. **Given** un archivo tiene cambios pendientes, **When** el usuario hace clic derecho y selecciona "Sincronizar ahora", **Then** se inicia la sincronización inmediata de ese archivo.
4. **Given** el usuario selecciona múltiples archivos, **When** hace clic derecho, **Then** las acciones de LNXDrive aplican a toda la selección.
5. **Given** un archivo no pertenece al directorio de LNXDrive, **When** el usuario hace clic derecho, **Then** no aparece el submenú de LNXDrive.

---

### User Story 3 - Indicador de estado en la barra superior de GNOME (Priority: P2)

El usuario ve un icono de LNXDrive en la barra superior de GNOME Shell que refleja el estado global de sincronización: inactivo, sincronizando, con errores, o sin conexión. Al hacer clic en el icono, se despliega un menú compacto que muestra el progreso de sincronización actual (si hay una en curso), la cantidad de archivos pendientes, acceso rápido a conflictos pendientes, información de cuota de almacenamiento utilizada, y opciones rápidas para pausar/reanudar la sincronización.

**Why this priority**: El indicador en la barra superior da visibilidad persistente y global del estado de sincronización sin necesidad de abrir Nautilus. Es importante pero secundario frente a la integración directa en el gestor de archivos.

**Independent Test**: Puede probarse verificando que el icono en la barra superior refleja correctamente el estado global y que el menú desplegable muestra información actualizada en tiempo real. Entrega valor al dar visibilidad constante del estado de la sincronización.

**Acceptance Scenarios**:

1. **Given** la sincronización está en reposo y no hay errores, **When** el usuario mira la barra superior, **Then** ve el icono de LNXDrive en estado "idle" (inactivo).
2. **Given** hay una sincronización en curso, **When** el usuario hace clic en el icono, **Then** el menú muestra el archivo actual, el progreso (porcentaje o barra) y la cantidad de archivos restantes.
3. **Given** hay conflictos de sincronización pendientes, **When** el usuario hace clic en el icono, **Then** el menú muestra el número de conflictos y una opción para verlos.
4. **Given** no hay conexión a Internet, **When** el usuario mira la barra superior, **Then** el icono refleja el estado "sin conexión" y el menú lo confirma.
5. **Given** el usuario hace clic en "Pausar sincronización", **When** la acción se ejecuta, **Then** la sincronización se detiene, el icono cambia a estado "pausado", y aparece la opción "Reanudar sincronización".
6. **Given** el usuario hace clic en "Información de cuota", **When** se despliega, **Then** se muestra el espacio utilizado y disponible en la cuenta de nube.

---

### User Story 4 - Panel de preferencias de LNXDrive (Priority: P2)

El usuario accede al panel de configuración de LNXDrive desde la extensión de GNOME Shell (opción "Preferencias") o desde la aplicación de configuración del sistema. El panel permite configurar: selección de carpetas a sincronizar (sincronización selectiva con vista de árbol), patrones de exclusión de archivos de forma visual, políticas de resolución de conflictos, comportamiento de la sincronización (automática o manual), y límites de ancho de banda. La interfaz sigue las directrices de diseño de GNOME (HIG) utilizando los componentes estándar de la plataforma.

**Why this priority**: La configuración es esencial para personalizar la experiencia, pero los valores por defecto razonables permiten que el sistema funcione sin ella. Por eso es P2: importante, pero el producto es usable sin esta pantalla.

**Independent Test**: Puede probarse abriendo el panel de preferencias, modificando la configuración (por ejemplo, excluyendo una carpeta de la sincronización) y verificando que el cambio se refleja en el comportamiento real de la sincronización.

**Acceptance Scenarios**:

1. **Given** el usuario abre el panel de preferencias, **When** navega a la sección de sincronización selectiva, **Then** ve un árbol de carpetas con checkboxes que reflejan qué carpetas están sincronizadas.
2. **Given** el usuario desmarca una carpeta en el árbol, **When** guarda los cambios, **Then** esa carpeta deja de sincronizarse y los archivos locales se convierten en placeholders.
3. **Given** el usuario añade un patrón de exclusión (por ejemplo, "*.tmp"), **When** guarda, **Then** los archivos que coinciden con ese patrón dejan de sincronizarse.
4. **Given** el usuario cambia la política de conflictos a "mantener versión local", **When** ocurre un conflicto posterior, **Then** el sistema aplica automáticamente la política configurada.
5. **Given** el usuario establece un límite de ancho de banda, **When** la sincronización está activa, **Then** la velocidad de transferencia respeta el límite configurado.

---

### User Story 5 - Primer arranque y configuración inicial (Priority: P1)

Un usuario que acaba de instalar LNXDrive abre la aplicación por primera vez (o es redirigido automáticamente al detectar que no hay cuenta configurada). Se presenta un asistente de configuración inicial (wizard) de 2-3 pasos: primero, autenticación con su cuenta Microsoft (OAuth2 con PKCE); segundo, selección de la carpeta local donde se montará la sincronización; tercero, confirmación y arranque de la primera sincronización. El asistente sigue las directrices de diseño de GNOME y es el punto de entrada principal para usuarios nuevos.

**Why this priority**: Sin una cuenta configurada, ningún otro componente de la integración GNOME tiene funcionalidad. El onboarding es pre-requisito para que el usuario pueda usar overlay icons, menú contextual, indicador de estado o cualquier otra funcionalidad. Es P1 porque desbloquea todo lo demás.

**Independent Test**: Puede probarse ejecutando la aplicación en un entorno limpio (sin cuenta previa) y verificando que el wizard guía al usuario hasta una sincronización inicial funcional.

**Acceptance Scenarios**:

1. **Given** LNXDrive está recién instalado y no hay cuenta configurada, **When** el usuario abre la aplicación o la extensión de Shell detecta la ausencia de cuenta, **Then** se lanza automáticamente el asistente de primer arranque.
2. **Given** el asistente muestra el paso de autenticación, **When** el usuario completa el flujo OAuth2 con su cuenta Microsoft, **Then** el asistente avanza al siguiente paso con confirmación visual de cuenta conectada.
3. **Given** el asistente muestra el paso de selección de carpeta, **When** el usuario elige una carpeta local, **Then** la carpeta se configura como punto de montaje de sincronización.
4. **Given** el usuario completa todos los pasos del asistente, **When** confirma la configuración, **Then** la primera sincronización se inicia automáticamente y el usuario es redirigido a Nautilus o al indicador de estado.
5. **Given** el usuario cancela el asistente a mitad del proceso, **When** vuelve a abrir LNXDrive, **Then** el asistente se presenta de nuevo desde el principio sin datos parciales guardados.

---

### User Story 6 - Inicio de sesión mediante GNOME Online Accounts (Priority: P3)

Un usuario que quiere configurar LNXDrive por primera vez va a Configuración del Sistema → Cuentas en Línea, donde encuentra la opción "Microsoft (LNXDrive)". Al seleccionarla, se abre un flujo de autenticación OAuth2 donde el usuario inicia sesión con su cuenta de Microsoft. Una vez autenticado, LNXDrive comienza a sincronizar automáticamente. Si el usuario ya tiene una cuenta de Microsoft configurada en GNOME Online Accounts, LNXDrive puede reutilizar esas credenciales existentes (single sign-on).

**Why this priority**: Aunque la integración con GNOME Online Accounts es la forma más "nativa" de autenticación en GNOME, el sistema puede funcionar con una autenticación independiente a través de la CLI o el panel de preferencias. Esta integración añade pulido y experiencia nativa, pero no es bloqueante para el uso del producto.

**Independent Test**: Puede probarse añadiendo una cuenta Microsoft a través de GNOME Online Accounts y verificando que LNXDrive la detecta y comienza la sincronización sin pedir credenciales adicionales.

**Acceptance Scenarios**:

1. **Given** el usuario abre GNOME Online Accounts, **When** selecciona "Microsoft (LNXDrive)", **Then** se inicia un flujo de autenticación OAuth2 en una ventana segura.
2. **Given** el usuario completa la autenticación correctamente, **When** vuelve a la pantalla de cuentas, **Then** la cuenta aparece como conectada y LNXDrive inicia la sincronización automáticamente.
3. **Given** el usuario ya tiene una cuenta Microsoft configurada en GNOME, **When** instala LNXDrive, **Then** se le ofrece reutilizar la cuenta existente sin repetir el flujo de autenticación.
4. **Given** el token de autenticación ha expirado, **When** LNXDrive intenta sincronizar, **Then** renueva el token automáticamente sin intervención del usuario (refresh token).
5. **Given** el usuario elimina la cuenta desde GNOME Online Accounts, **When** vuelve a LNXDrive, **Then** la sincronización se detiene y el usuario es notificado.

---

### Edge Cases

- ¿Qué sucede cuando el daemon de LNXDrive no está corriendo y el usuario abre Nautilus? Los overlay icons deben indicar "estado desconocido" o no mostrarse, y el menú contextual debe informar que el servicio no está disponible.
- ¿Qué sucede si la conexión D-Bus se pierde mientras el usuario está interactuando con la extensión de GNOME Shell? La extensión debe mostrar un estado de "reconectando" y recuperarse automáticamente cuando la conexión se restablezca.
- ¿Qué sucede cuando hay miles de archivos en una carpeta? Los overlay icons y el menú contextual deben mantener su rendimiento sin degradar la experiencia de Nautilus.
- ¿Qué sucede si el usuario tiene múltiples monitores con diferentes escalas? Los iconos y la interfaz de preferencias deben renderizarse correctamente con HiDPI.
- ¿Qué sucede cuando el usuario actualiza GNOME a una nueva versión? La extensión de Shell debe ser compatible con versiones GNOME 45, 46 y 47 al menos.
- ¿Qué sucede si el usuario intenta "Liberar espacio" de un archivo que está siendo editado por otra aplicación? El sistema debe detectar el uso activo y rechazar la operación con un mensaje claro.
- ¿Qué sucede cuando no hay espacio en disco para hidratar un archivo? El sistema debe informar al usuario antes de comenzar la descarga o abortar con un mensaje claro.
- ¿Qué sucede si la extensión de Nautilus no puede comunicarse con el daemon al ejecutar una acción? La acción debe fallar con un mensaje comprensible, no silenciosamente.

## Requirements *(mandatory)*

### Functional Requirements

**Extensión de Nautilus - Overlay Icons y Columnas**

- **FR-001**: El sistema DEBE mostrar overlay icons en archivos y carpetas dentro del directorio sincronizado de LNXDrive que reflejen su estado actual de sincronización (sincronizado, solo en la nube, sincronizando, pendiente, conflicto, error, excluido, desconocido). El estado "desconocido" se muestra cuando el daemon no está disponible.
- **FR-002**: Los overlay icons DEBEN actualizarse en tiempo real cuando el estado de un archivo cambia, sin requerir que el usuario recargue la carpeta.
- **FR-003**: El sistema DEBE proveer columnas adicionales en Nautilus que muestren el estado de sincronización y la fecha de última sincronización.
- **FR-004**: Los overlay icons DEBEN ser visualmente distinguibles entre sí para cada uno de los estados posibles, siguiendo las convenciones de iconografía de GNOME.

**Extensión de Nautilus - Menú Contextual**

- **FR-005**: El sistema DEBE agregar un submenú "LNXDrive" al menú contextual de Nautilus exclusivamente para archivos y carpetas dentro del directorio sincronizado.
- **FR-006**: El submenú DEBE mostrar acciones contextuales adaptadas al estado actual del archivo seleccionado: "Mantener disponible offline" (pin), "Liberar espacio" (unpin/dehydrate), "Sincronizar ahora".
- **FR-007**: Las acciones del menú contextual DEBEN funcionar tanto con selección individual como con selección múltiple de archivos.
- **FR-008**: El sistema DEBE mostrar una respuesta visual (notificación o cambio de overlay icon) al usuario cuando se ejecuta una acción desde el menú contextual.

**Extensión GNOME Shell - Indicador de Estado**

- **FR-009**: El sistema DEBE mostrar un icono persistente en la barra superior de GNOME Shell que refleje el estado global de sincronización (inactivo, sincronizando, pausado, error, sin conexión).
- **FR-010**: El menú desplegable del indicador DEBE mostrar: progreso de sincronización actual, número de archivos pendientes, número de conflictos pendientes, información de cuota de almacenamiento.
- **FR-011**: El menú desplegable DEBE incluir acciones rápidas: pausar/reanudar sincronización, sincronizar ahora, y acceso al panel de preferencias.
- **FR-012**: La extensión DEBE ser compatible con GNOME Shell versiones 45, 46 y 47 como mínimo.

**Extensión de Nautilus - Restricción de API**

- **FR-030**: La extensión de Nautilus DEBE implementarse exclusivamente usando libnautilus-extension-4 (API nativa GTK4). No se soporta nautilus-python ni la API legacy de extensiones.

**Panel de Preferencias**

- **FR-013**: El sistema DEBE proveer un panel de preferencias con interfaz gráfica que siga las directrices de diseño de GNOME (HIG).
- **FR-014**: El panel DEBE permitir configurar sincronización selectiva mediante una vista de árbol de carpetas con checkboxes.
- **FR-015**: El panel DEBE permitir definir patrones de exclusión de archivos de forma visual (añadir, editar, eliminar patrones).
- **FR-016**: El panel DEBE permitir configurar la política de resolución de conflictos (preguntar siempre, mantener local, mantener remoto, mantener ambos).
- **FR-017**: El panel DEBE permitir configurar límites de ancho de banda para subida y bajada.
- **FR-018**: El panel DEBE permitir configurar el comportamiento de sincronización (automática o manual).

**Primer Arranque (Onboarding)**

- **FR-031**: Cuando no exista una cuenta configurada, el sistema DEBE lanzar automáticamente un asistente de primer arranque (wizard) que guíe al usuario por los pasos: autenticación, selección de carpeta y confirmación.
- **FR-032**: El asistente DEBE ser independiente de GNOME Online Accounts, utilizando su propio flujo OAuth2 con PKCE para la autenticación.
- **FR-033**: Si el usuario cancela el asistente, el sistema NO DEBE guardar datos parciales de configuración y DEBE volver a presentar el asistente en el siguiente arranque.
- **FR-034**: Al completar el asistente, el sistema DEBE iniciar la primera sincronización automáticamente.

**Integración con GNOME Online Accounts**

- **FR-019**: El sistema DEBE registrar un proveedor de cuenta en GNOME Online Accounts que permita autenticación con cuentas Microsoft.
- **FR-020**: El flujo de autenticación DEBE utilizar OAuth2 con PKCE a través de una ventana segura integrada.
- **FR-021**: El sistema DEBE poder reutilizar una cuenta Microsoft ya configurada en GNOME Online Accounts para evitar doble autenticación.
- **FR-022**: El sistema DEBE renovar automáticamente los tokens de acceso cuando expiren, sin intervención del usuario.
- **FR-023**: Cuando el usuario elimina la cuenta desde GNOME Online Accounts, el sistema DEBE detener la sincronización y notificar al usuario.

**Comunicación con el Daemon**

- **FR-024**: Toda la comunicación entre los componentes de la interfaz GNOME y el daemon DEBE realizarse a través de la API D-Bus definida por el proyecto (`org.enigmora.LNXDrive`).
- **FR-025**: Los componentes de la interfaz DEBEN manejar graciosamente la desconexión del daemon, mostrando un estado apropiado y reconectándose automáticamente cuando el daemon vuelva a estar disponible.
- **FR-026**: Los componentes DEBEN suscribirse a las señales D-Bus para recibir actualizaciones en tiempo real en lugar de hacer polling.

**Resiliencia y Rendimiento**

- **FR-027**: La extensión de Nautilus NO DEBE degradar el rendimiento de Nautilus cuando hay carpetas con más de 5000 archivos. Específicamente, los overlay icons y el menú contextual DEBEN responder en ≤500ms (ver SC-005).
- **FR-028**: La extensión de GNOME Shell NO DEBE consumir más recursos que una extensión típica de la plataforma en estado inactivo.
- **FR-029**: Los componentes de la interfaz DEBEN soportar pantallas HiDPI y escalado fraccionario.
- **FR-035**: Todas las cadenas de texto visibles al usuario DEBEN estar preparadas para internacionalización (i18n) mediante gettext. El idioma base DEBE ser inglés. Las traducciones adicionales se incorporan incrementalmente sin requerir cambios en el código.

**Manejo de Errores en Operaciones de Archivos**

- **FR-036**: Cuando el usuario solicita hidratar (pin) un archivo y no hay espacio suficiente en disco, el sistema DEBE abortar la operación antes de iniciar la descarga y mostrar un mensaje de error claro indicando el espacio requerido y el disponible.
- **FR-037**: Cuando el usuario solicita deshidratar (unpin/liberar espacio) un archivo que está siendo utilizado activamente por otra aplicación, el sistema DEBE rechazar la operación y mostrar un mensaje indicando que el archivo está en uso.

### Key Entities

- **FileOverlayState**: Estado visual de un archivo en Nautilus, derivado de su estado de sincronización real. Mapea estados del daemon (synced, cloud-only, syncing, pending, conflict, error, excluded) a iconos y etiquetas visuales.
- **SyncStatusSummary**: Resumen del estado global de sincronización consumido por la extensión de GNOME Shell. Incluye: estado general, archivos en progreso, archivos pendientes, conflictos activos, información de cuota.
- **UserPreferences**: Conjunto de configuraciones del usuario gestionadas desde el panel de preferencias. Incluye: carpetas seleccionadas, patrones de exclusión, política de conflictos, límites de ancho de banda, modo de sincronización.
- **AccountConnection**: Relación entre una cuenta de GNOME Online Accounts y la configuración de sincronización de LNXDrive. Incluye: estado de autenticación, tokens activos, estado de la cuenta.

## Clarifications

### Session 2026-02-05

- Q: ¿Qué API de extensión de Nautilus se debe soportar (libnautilus-extension-4 nativa GTK4, nautilus-python legacy, o ambas)? → A: Solo libnautilus-extension-4 (API nativa GTK4). No se soporta nautilus-python ni la API legacy.
- Q: ¿Qué experiencia de primer arranque (onboarding) tiene el usuario que instala LNXDrive por primera vez? → A: Asistente de primer arranque propio en el panel de preferencias (wizard de 2-3 pasos: autenticación, selección de carpeta, confirmar). No depende de GNOME Online Accounts.
- Q: ¿Qué estrategia de localización/idioma se usa para la UI? → A: Inglés como idioma base con soporte i18n habilitado (gettext). Las traducciones se agregan incrementalmente.

## Assumptions

- El daemon de LNXDrive (`lnxdrive-daemon`) y la API D-Bus (`org.enigmora.LNXDrive`) ya existen y están funcionales (entregados en Fases 1-2).
- El sistema FUSE de files-on-demand ya está operativo, proporcionando los estados de archivo que la extensión de Nautilus representará visualmente (entregado en Fase 2).
- La librería IPC compartida (`lnxdrive-ipc`) ya proporciona un cliente D-Bus tipado para comunicación con el daemon.
- El usuario objetivo usa GNOME como entorno de escritorio principal con Nautilus como gestor de archivos.
- La extensión de GNOME Shell se distribuirá como paquete del sistema (no a través de extensions.gnome.org inicialmente), para simplificar el ciclo de entrega.
- La configuración declarativa en YAML definida por el proyecto es la fuente de verdad para las preferencias; el panel de preferencias es una interfaz gráfica sobre esa configuración.
- Se prioriza GNOME 45+ como línea base de compatibilidad, alineado con las distribuciones LTS actuales (Ubuntu 24.04, Fedora 40+).

## Dependencies

- **Fase 1 (Core + CLI)**: Provee el daemon, la API D-Bus, y el motor de sincronización.
- **Fase 2 (Files-on-Demand)**: Provee el sistema FUSE con estados de archivo que la UI representará.
- **lnxdrive-ipc crate**: Provee el cliente D-Bus tipado para comunicación Rust con el daemon.
- **GNOME Platform**: GTK4, libadwaita, GNOME Shell, Nautilus, GNOME Online Accounts (versiones 45+).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: El usuario puede identificar visualmente el estado de sincronización de cualquier archivo en Nautilus en menos de 2 segundos al navegar a una carpeta sincronizada.
- **SC-002**: El usuario puede ejecutar acciones de sincronización (pin, unpin, sync) desde el menú contextual de Nautilus en un máximo de 2 clics (clic derecho + selección de opción).
- **SC-003**: El indicador de la barra superior refleja cambios de estado de sincronización dentro de los 3 segundos posteriores al cambio real.
- **SC-004**: El usuario puede completar la configuración inicial (autenticación + selección de carpetas) en menos de 5 minutos.
- **SC-005**: La extensión de Nautilus mantiene su tiempo de respuesta (overlay icons y menú contextual) por debajo de 500 milisegundos incluso en carpetas con más de 5000 archivos.
- **SC-006**: El 95% de los usuarios de GNOME pueden completar las tareas básicas (ver estado, pin/unpin, pausar sincronización) sin consultar documentación.
- **SC-007**: La extensión de GNOME Shell funciona sin errores en al menos 3 versiones consecutivas de GNOME (45, 46, 47).
- **SC-008**: Tras una desconexión del daemon, los componentes de UI se recuperan automáticamente en menos de 10 segundos al restaurarse la conexión.
