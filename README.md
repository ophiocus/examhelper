# ExamHelper — Naturalización Colombia

![ExamHelper Screenshot](assets/screenshot.png)

Aplicación de escritorio para Windows que te prepara para el **examen de naturalización colombiana** (categoría padre/madre de nacional). Construida en Rust con egui.

---

## Características

### Modo Estudio
- 15 temas organizados en 4 categorías: **Constitución**, **Historia**, **Geografía** y **Cultura**
- Contenido renderizado en Markdown con navegación por panel lateral
- Seguimiento de progreso — marca cada tema como leído

### Narración por Voz (TTS)
- Narración integrada usando voces nativas de Windows (SAPI/WinRT)
- Selector de voz en la barra de reproducción — elige entre todas las voces instaladas
- Control de velocidad: 0.25× a 2.0× con slider visual
- Modo **Auto**: narra automáticamente al seleccionar un nuevo tema
- Hilo de fondo — nunca bloquea la interfaz

### Modo Examen
- **117 preguntas fijas** + **11 plantillas dinámicas** con 122+ pares = **239+ preguntas únicas**
- Las plantillas generan variantes diferentes cada vez (ej: capitales de los 32 departamentos)
- Selección de categorías y cantidad de preguntas por categoría (5–30)
- Navegador de preguntas, barra de progreso, revisión completa con correcciones
- Calificación por categoría con indicador aprobado/reprobado (≥70%)

### Configuración
- Gestión de voces TTS — ver voces instaladas, seleccionar activa
- Instalación de paquetes de voz de Windows desde la app (elevación automática)
- Modo claro/oscuro
- Zoom global con presets (75%–200%) y slider arrastrable en la barra de estado
- Persistencia de configuración y progreso en `%APPDATA%\ExamHelper\`

### Actualización de Contenido
- Contenido almacenado en archivos Markdown y TOML — editable y versionable
- Botón **"Actualizar Contenido"** ejecuta `git pull` para sincronizar con un repositorio remoto
- Agrega temas o preguntas sin recompilar — la app los detecta al reiniciar

---

## Contenido del Examen

| Categoría | Temas | Preguntas |
|-----------|-------|-----------|
| **Constitución** | Estructura del Estado, Ramas del Poder, Derechos Fundamentales, Participación Democrática, Nacionalidad (Art. 96) | 34 fijas + 19 dinámicas |
| **Historia** | Época Precolombina, Independencia, República, Personajes Importantes | 31 fijas + 20 dinámicas |
| **Geografía** | Datos Generales, Regiones Naturales, 32 Departamentos y Capitales | 21 fijas + 64 dinámicas |
| **Cultura** | Símbolos Nacionales, Himno Nacional, Fiestas y Patrimonio | 31 fijas + 19 dinámicas |

---

## Requisitos

- Windows 10/11
- Voz en español instalada (la app puede instalarla desde **Config**)

## Compilar

```bash
cargo build --release
```

El binario queda en `target\release\examhelper.exe` (~8 MB).

## Ejecutar

```bash
# Desde el directorio del proyecto:
cargo run

# O directamente:
target\release\examhelper.exe
```

## Estructura del Proyecto

```
examhelper/
├── src/
│   └── main.rs              # Aplicación completa (~2800 líneas)
├── content/                  # Material de estudio (Markdown)
│   ├── 01-constitucion/      # 5 temas
│   ├── 02-historia/          # 4 temas
│   ├── 03-geografia/         # 3 temas
│   └── 04-cultura/           # 3 temas
├── questions/                # Bancos de preguntas (TOML)
│   ├── constitucion.toml
│   ├── historia.toml
│   ├── geografia.toml
│   └── cultura.toml
├── assets/
│   └── screenshot.png        # Screenshot de la app
├── Cargo.toml
├── build.rs
└── README.md
```

## Agregar Contenido

**Nuevo tema de estudio:** crea un archivo `.md` en la carpeta correspondiente dentro de `content/`.

**Nuevas preguntas fijas:**
```toml
[[questions]]
text = "¿Pregunta aquí?"
options = ["Opción A", "Opción B", "Opción C", "Opción D"]
answer = 0  # índice base cero de la respuesta correcta
```

**Nuevas preguntas dinámicas (plantillas):**
```toml
[[templates]]
text = "¿Cuál es la capital de {key}?"
variable = "key"
answer_template = "{value}"
distractors = ["Bogotá", "Medellín", "Cali", "Barranquilla"]

[[templates.pairs]]
key = "Antioquia"
value = "Medellín"

[[templates.pairs]]
key = "Boyacá"
value = "Tunja"
```

---

## Licencia

MIT

## Autor

ophiocus
