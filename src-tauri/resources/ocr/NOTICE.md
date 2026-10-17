# Lectura local de símbolos

Screen-Pin incluye Tesseract 5.5.2 y el modelo inglés sin modificar de
[tessdata_best](https://github.com/tesseract-ocr/tessdata_best), bajo Apache 2.0
(archivo LICENSE). Tesseract usa Leptonica 1.87.0, cuya licencia está en
LICENSE-Leptonica.txt. Los enlaces Rust tesseract-rs 0.4.0 se distribuyen bajo
la licencia MIT incluida en LICENSE-tesseract-rs.txt.

Procedencia del modelo eng.traineddata:

- Revisión: e12c65a915945e4c28e237a9b52bc4a8f39a0cec.
- Fuente: https://github.com/tesseract-ocr/tessdata_best/blob/e12c65a915945e4c28e237a9b52bc4a8f39a0cec/eng.traineddata
- SHA-256: 8280aed0782fe27257a68ea10fe7ef324ca0f8d85bd2fd145d1c2b560bcb66ba.

El modelo está integrado en el ejecutable. La aplicación no descarga modelos
ni envía capturas a servicios externos durante el reconocimiento.
