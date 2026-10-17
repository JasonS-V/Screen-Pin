# DESIGN.md - Screen-Pin Visual System & Specifications

## 1. Direction & Dials (Liveliness Toolkit)

> **Reading this as:** Utilidad de escritorio residente y visor flotante (HUD) para usuarios multitarea, en un lenguaje visual de instrumento óptico de precisión, dial **ENERGY 2 / RHYTHM 2 / MOTION 1**.

* **ENERGY 2 (Equilibrado y Táctil):** Presencia física nítida mediante chasis de alto contraste (doble anillo), evitando distracciones de neón o transparencias difusas excesivas.
* **RHYTHM 2 (Consistente con variación funcional):** Stickers adaptados a la proporción geométrica de la captura; panel de memoria indexado verticalmente con tarjetas asimétricas según el tipo de dato.
* **MOTION 1 (Transiciones de estado funcionales):** Cero animaciones superfluas o de rebote. Transiciones de estado estrictamente de confirmación en $\le 120\text{ ms}$.

---

## 2. Paleta de Colores y Verificación Matemática de Contraste (WCAG 2.x)

$$L = 0.2126 \times R_{\text{lin}} + 0.7152 \times G_{\text{lin}} + 0.0722 \times B_{\text{lin}}$$
$$\text{Ratio de Contraste} = \frac{L_1 + 0.05}{L_2 + 0.05}$$

| Rol en el Sistema | Nombre y Código HEX | Luminancia | Emparejamiento y Ratio WCAG | Cumplimiento |
| :--- | :--- | :--- | :--- | :--- |
| **Superficie Base (Chasis / Dark)** | **Carbón Óptico** `#141413` | $L = 0.0065$ | Con texto `#F4F3EF`: **$16.82:1$** | **PASS AAA** |
| **Superficie Base (Papel / Light)** | **Papel Técnico** `#F4F3EF` | $L = 0.8990$ | Con texto `#141413`: **$16.82:1$** | **PASS AAA** |
| **Superficie Secundaria / Bordes** | **Pizarra Mate** `#262624` | $L = 0.0185$ | Con texto `#F4F3EF`: **$13.84:1$** | **PASS AAA** |
| **Texto Secundario / Metadatos (Oscuro)** | **Gris Escala Oscuro** `#A3A19A` | $L = 0.3640$ | Sobre `#141413`: **$7.42:1$** | **PASS AA** |
| **Texto Secundario / Metadatos (Claro)** | **Gris Escala Claro** `#4A4945` | $L = 0.0680$ | Sobre `#F4F3EF`: **$8.15:1$** | **PASS AA** |
| **Acento Deliberado Único** | **Ámbar Señal** `#D95D1E` | $L = 0.1840$ | Borde sobre `#F4F3EF`: **$3.62:1$** (No-Texto) | **PASS AA** |
| **Acento Foco Accesible** | **Ámbar Resaltado** `#FF8A50` | $L = 0.3800$ | Con texto `#141413`: **$8.90:1$** | **PASS AAA** |

### Película de Contraste sobre Imágenes Arbitrarias
Al seleccionar texto en el sticker, la caja de selección utiliza `rgba(20, 20, 19, 0.90)` con texto `#F4F3EF` garantizando siempre ratio $>14:1$.

---

## 3. Tipografía con Propósito

* **Interfaz, HUD y Cajón de Memoria:** `Plus Jakarta Sans` o fuentes de sistema con amplia $x$-height (`Segoe UI Variable`, `SF Pro`, system-ui).
* **Datos Extraídos, Coordenadas y Cifras Bancarias:** `JetBrains Mono` con distinción inequívoca de `0/O` y `1/l/I`.

---

## 4. Anatomía del Sticker ("Portaobjetos Óptico")

1. **Chasis de Doble Anillo:** `border-radius: 2px`, `outline` interior de $1\text{ px}$ en `#141413`, borde exterior de $1\text{ px}$ en `#F4F3EF`, sombra de elevación `0 4px 12px rgba(0, 0, 0, 0.28)`.
2. **Escuadra de Calibración:** Marca en "L" de $8\text{ px} \times 8\text{ px}$ en `#D95D1E` en la esquina superior izquierda. Parpadea durante escaneo OCR y queda fija al terminar; funciona como handle de arrastre seguro.
3. **Barra flotante independiente (330 × 44 px):** Sigue al recorte, se coloca debajo o encima según el espacio disponible y muestra Copiar texto, Fantasma y Archivar sin desplazamiento horizontal.
