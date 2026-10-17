# Microsoft Visual C++ Runtime (Windows x64)

Copyright Microsoft Corporation. Todos los derechos reservados.

La distribución Windows x64 incluye las bibliotecas redistribuibles
msvcp140.dll, vcruntime140.dll y vcruntime140_1.dll necesarias para Tesseract.
Se copian junto al ejecutable, sin modificar ni registrar bibliotecas del sistema.

Procedencia: Visual Studio 2022 Community, VC/Redist/MSVC/14.44.35112/x64/Microsoft.VC143.CRT.
Se verificaron las firmas Authenticode de Microsoft antes de copiarlas.
La lista de redistribución incluida por Visual Studio está en Redist.txt.

- [Archivos redistribuibles](https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution)
- [Distribución de las bibliotecas](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files)

Al actualizar las herramientas de C++ o Tesseract, comprobar sus dependencias
y actualizar juntas estas bibliotecas desde la carpeta Redist de Visual Studio.
Para compilar para otra arquitectura, sustituirlas por sus equivalentes de esa arquitectura.
