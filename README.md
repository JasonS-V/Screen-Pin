# Screen-Pin

Utilidad de escritorio para Windows: recorta una zona de la pantalla, mantenla siempre encima y recupera su imagen o texto desde la memoria local.

## Desarrollo

Necesitas Node.js 24, pnpm, Rust estable, CMake 3.31 o posterior, las herramientas de C++ de Visual Studio y WebView2. La primera compilación descarga y compila Tesseract y Leptonica; las siguientes reutilizan su caché. CMake debe estar en el PATH o indicado mediante la variable `CMAKE`. El OCR utiliza los idiomas de reconocimiento instalados en Windows y un modelo inglés integrado para apoyar la lectura de símbolos.

```sh
pnpm install --frozen-lockfile
pnpm tauri dev
```

`pnpm dev` abre únicamente la vista previa web. Las capturas, la memoria y los atajos requieren la aplicación de escritorio; la vista previa lo indica y desactiva las acciones nativas.

## Uso

- **Alt + Shift + S**: congela los monitores. Arrastra una zona para fijarla; **Esc** o clic derecho cancela.
- **Flechas**: mueve el cursor de selección en pasos de 10 píxeles lógicos; **Shift + flechas**, de uno en uno. **Espacio** fija el ancla y **Enter** confirma.
- El recorte permanece encima de otras ventanas. Arrastra desde la esquina ámbar o una zona sin texto. El texto reconocido se puede seleccionar y copiar; los controles aparecen al pasar el cursor o usar **Tab**.
- **Archivar**, **Esc** o doble clic sobre una zona sin texto guarda el recorte en memoria y cierra su ventana.
- **Alt + Shift + M**: abre Memoria. Busca por el texto reconocido, copia el contenido o vuelve a fijar una captura. Volver a fijar un recorte ya abierto enfoca su ventana.
- **Fantasma** permite que los clics atraviesen el recorte. **Alt + Shift + G**, abrir Memoria o el menú de la bandeja recupera la interacción.
- Cerrar la ventana principal deja Screen-Pin en la bandeja. El menú de la bandeja permite abrirla de nuevo o salir de la aplicación.

Si otra aplicación ocupa un atajo, Screen-Pin lo indica al iniciar y mantiene disponibles los botones y el menú de la bandeja.

El OCR amplía la imagen y combina una lectura con contraste local con otra en color para recuperar texto pequeño o tenue en fondos claros y oscuros. Tesseract complementa al motor de Windows en líneas con rutas, variables y operadores de consola o código, como `C:\Usuarios>`, `|`, `<=`, `!=`, `&&`, `foo_bar` y `%PATH%`. El modelo adicional está integrado en la aplicación y funciona sin conexión. Se mantiene la lectura de Windows cuando una línea contiene letras de otros idiomas que el lector adicional podría perder.

Las capturas grandes se leen por zonas solapadas, respetando el [límite de tamaño del motor de Windows](https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr.ocrengine.maximagedimension). La imagen guardada conserva su aspecto original. Algunos símbolos y combinaciones de contraseñas pueden seguir omitiéndose o confundirse; conviene revisar esos caracteres antes de copiarlos. Los recortes existentes conservan su lectura anterior; las mejoras se aplican a nuevas capturas. La procedencia y las licencias del lector adicional se encuentran en `src-tauri/resources/ocr/NOTICE.md` y se incluyen en la distribución.

## Datos y privacidad

Las imágenes PNG y la base SQLite se guardan en `%APPDATA%/ScreenPin`. El OCR se procesa en el equipo. Los recortes clasificados como cuentas, tarjetas o credenciales caducan a los 30 minutos; la aplicación comprueba los vencimientos al iniciar y cada 60 segundos mientras está abierta. La limpieza elimina el registro, intenta eliminar el PNG y libera el recorte abierto y su copia en RAM.

**Conservar** evita la caducidad de un recorte sensible. **Liberar** inicia un nuevo período de 30 minutos. La clasificación es automática y puede fallar; no es una garantía de detección ni de borrado seguro del disco. Los archivos locales no están cifrados.

La búsqueda muestra hasta 50 coincidencias y el listado inicial, los 50 recortes más recientes. Las bases de versiones anteriores se migran automáticamente y se reconstruye el índice de búsqueda.

## Verificación y compilación

```sh
pnpm check
pnpm test
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
pnpm build
pnpm tauri build
```

Las pruebas cubren coordenadas y límites del recorte, clasificación, búsqueda y actualización de su índice, migración de bases existentes, retención, conservación del archivado durante el OCR y acceso directo por identificador. Las pruebas de Windows ejecutan los motores reales sobre imágenes sintéticas con texto pequeño, grises de bajo contraste, números largos, correos, importes, porcentajes, signos de consola en fondos claros y oscuros y acentos en español, además de capturas mayores que el límite y palabras en el solapamiento de dos zonas.

Para una compilación nativa de prueba sin generar un instalador: `pnpm tauri build --debug --no-bundle`.

La distribución Windows x64 incluye junto al ejecutable las bibliotecas de Microsoft C++ que necesita Tesseract. Conserva los archivos de esa carpeta al mover una compilación sin instalador. Su procedencia y mantenimiento se documentan en `src-tauri/resources/runtime/windows-x64/NOTICE.md`; al compilar para otra arquitectura deben sustituirse por las bibliotecas correspondientes.

En compilaciones de desarrollo, `SCREEN_PIN_DATA_DIR` permite usar una carpeta de pruebas independiente. Las compilaciones de distribución siempre utilizan la carpeta normal.

## Identidad visual

La fuente del favicon y los iconos es `static/favicon.svg`. Para regenerar los iconos de Tauri: `pnpm tauri icon static/favicon.svg`. La interfaz utiliza fuentes del sistema y conserva la paleta carbón, papel y ámbar definida en `DESIGN.md`.
