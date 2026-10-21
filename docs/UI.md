# UI y design system

Este documento define cómo debe construirse la interfaz de VideoGeneratorAI y cómo reutilizar la misma base visual en futuras aplicaciones.

La UI debe sentirse como una **aplicación de escritorio nativa y profesional**, no como una página web metida dentro de una ventana.

---

## 1. Objetivo

Queremos un sistema visual reutilizable que permita:

- compartir lenguaje visual entre aplicaciones;
- evitar repetir widgets;
- mantener consistencia;
- reemplazar dependencias visuales sin reescribir las apps;
- permitir identidad propia por aplicación;
- separar completamente UI de lógica de negocio.

La aplicación debe depender de **nuestro UI kit**, no de librerías externas repartidas por todo el código.

---

## 2. Arquitectura visual

```text
egui / eframe
      │
      ├── shadcn-egui     (opcional / a evaluar)
      ├── egui_dock       (opcional / a evaluar)
      ├── egui_extras     (opcional)
      └── craft-ui        (referencia o piezas evaluables)
               │
               ▼
      VideoGeneratorAI UI Kit
               │
               ▼
        Features de la app
```

Regla principal:

> Las features de la aplicación no deben depender directamente de implementaciones visuales externas si podemos encapsularlas detrás del UI kit.

---

## 3. UI kit propio

Crear un crate reutilizable, inicialmente dentro del repositorio y extraíble a un repo propio cuando exista una segunda aplicación que lo necesite.

Estructura conceptual:

```text
crates/
└── ui-kit/
    ├── src/
    │   ├── lib.rs
    │   ├── tokens/
    │   │   ├── colors.rs
    │   │   ├── spacing.rs
    │   │   ├── typography.rs
    │   │   ├── radius.rs
    │   │   └── sizes.rs
    │   ├── theme/
    │   │   ├── dark.rs
    │   │   ├── light.rs
    │   │   └── mod.rs
    │   ├── components/
    │   │   ├── button.rs
    │   │   ├── input.rs
    │   │   ├── slider.rs
    │   │   ├── tabs.rs
    │   │   ├── dialog.rs
    │   │   ├── tooltip.rs
    │   │   └── mod.rs
    │   ├── layout/
    │   │   ├── dock.rs
    │   │   ├── split.rs
    │   │   └── mod.rs
    │   └── icons/
    │       └── mod.rs
```

No convertir todavía el UI kit en un proyecto independiente. Primero debe demostrar reutilización real.

---

## 4. Qué pertenece al UI kit

Componentes genéricos reutilizables:

- botones;
- icon buttons;
- inputs;
- text areas;
- labels;
- sliders;
- toggles;
- checkboxes;
- selects;
- tabs;
- badges;
- cards;
- dialogs;
- popovers;
- tooltips;
- separators;
- split panes;
- docking;
- status bars;
- progress;
- loading states;
- empty states;
- error states;
- menús;
- iconografía;
- design tokens;
- temas.

---

## 5. Qué NO pertenece al UI kit

Componentes específicos de dominio deben permanecer en la aplicación hasta demostrar reutilización.

Ejemplos de VideoGeneratorAI:

```text
video_preview
timeline
generation_card
gpu_meter
reference_slot
model_card
job_row
prompt_editor
engine_status
```

Si otra aplicación necesita realmente uno de esos componentes, recién entonces evaluar moverlo al UI kit general.

Regla:

> No abstraer antes de tener al menos dos usos reales.

---

## 6. Dependencias externas

Podemos reutilizar proyectos existentes, pero detrás de nuestra API.

### shadcn-egui

Candidato para:

- controles básicos;
- themes;
- presets;
- componentes comunes;
- consistencia visual.

No adoptar automáticamente toda su API pública.

### egui_dock

Candidato para:

- docking;
- tabs;
- floating panels;
- drag & drop de paneles;
- layouts persistentes.

No reinventar docking salvo que una limitación concreta lo justifique.

### egui_extras

Usar componentes maduros cuando aporten valor.

### craft-ui

Tomarlo como:

- referencia de arquitectura;
- posible fuente de componentes reutilizables compatibles;
- ejemplo de cómo compartir UI entre múltiples apps egui.

No convertirlo automáticamente en nuestra identidad visual.

---

## 7. Regla de encapsulación

Correcto:

```rust
use videogen_ui::{
    Button,
    Dialog,
    DockArea,
    Icon,
    Theme,
};
```

Evitar que cada feature haga:

```rust
use shadcn_egui::...;
use egui_dock::...;
use craft_ui::...;
```

La capa propia nos permite cambiar implementaciones sin alterar toda la aplicación.

---

## 8. Design tokens

Todo el aspecto debe derivar de tokens.

Definir como mínimo:

- fondos;
- superficies;
- bordes;
- texto primario/secundario;
- colores de estado;
- accent;
- spacing;
- tamaños;
- radios;
- tipografía;
- densidad;
- alturas estándar;
- icon sizes.

Evitar valores visuales hardcodeados dispersos.

Ejemplo conceptual:

```text
tokens
├── colors
├── spacing
├── typography
├── radius
├── sizes
└── motion
```

---

## 9. Temas

El sistema debe permitir varias identidades sin cambiar componentes.

Inicialmente:

- Dark;
- Light.

Más adelante:

- Professional Dark;
- Studio Light;
- Compact;
- Touch/high-density variants si hacen falta.

Una aplicación puede elegir otro set de tokens sin duplicar widgets.

---

## 10. Filosofía visual

VideoGeneratorAI debe priorizar:

- jerarquía clara;
- alta densidad sin saturación;
- contenido multimedia como protagonista;
- controles avanzados accesibles pero no invasivos;
- feedback continuo;
- interfaces estables durante trabajos largos;
- lectura rápida del estado de GPU/modelo/job;
- drag & drop natural;
- keyboard shortcuts donde aporten velocidad.

Evitar:

- estética de dashboard web genérico;
- cards innecesarias para todo;
- padding excesivo;
- modales para acciones frecuentes;
- ocultar información crítica en tooltips;
- exponer términos internos del backend al usuario común.

---

## 11. Modelo mental de producto

El usuario debe pensar en **acciones**, no en backends.

Principal:

```text
Create
Compose
Character
Avatar
Sound
Enhance
Train
Library
Queue
Models
Settings
```

No exponer como navegación principal:

```text
LightX2V
OmniWeaving
ComfyUI
WanGP
Diffusers
```

Los engines quedan en configuración avanzada y selección automática.

---

## 12. Layout principal

Base conceptual:

```text
┌──────────────────────────────────────────────────────┐
│ VideoGeneratorAI                     GPU / Job state │
├────────────┬─────────────────────────────────────────┤
│ Projects   │                                         │
│ Create     │                                         │
│ Compose    │               WORKSPACE                 │
│ Character  │                                         │
│ Avatar     │                                         │
│ Sound      │                                         │
│ Enhance    │                                         │
│ Train      │                                         │
│            │                                         │
│ Library    │                                         │
│ Queue      │                                         │
│ Models     │                                         │
│ Settings   │                                         │
├────────────┴─────────────────────────────────────────┤
│ Job / stage / progress / VRAM / warnings             │
└──────────────────────────────────────────────────────┘
```

La navegación puede evolucionar, pero la separación entre navegación, workspace y estado global debe mantenerse.

---

## 13. Create

Create debe ser simple por defecto.

Mostrar:

- Text to Video / Image to Video;
- prompt;
- referencia principal;
- preset;
- resolución;
- duración;
- botón Generate.

Controles avanzados:

- seed;
- steps;
- guidance;
- engine;
- quantization;
- cache;
- offload;
- sampler;
- opciones específicas del backend.

Regla:

> La UI básica expresa intención. La UI avanzada expone implementación.

---

## 14. Compose

Pensada para OmniWeaving y futuros motores multimodales.

Debe permitir:

- texto;
- varias imágenes;
- video fuente;
- keyframes;
- referencias;
- drag & drop;
- orden temporal;
- reemplazo/composición.

La UI detecta la intención y construye el job correspondiente.

Ejemplo:

```text
texto
→ T2V

imagen
→ I2V

keyframes
→ interpolation

video
→ editing

video + imagen
→ reference/edit

varias imágenes
→ compositional generation
```

---

## 15. Library

Todo input/output se representa como un `MediaAsset`.

Metadata mínima:

```text
UUID
path
hash
media_type
width
height
frames
fps
duration
parent_job
prompt
model
seed
generation_parameters
```

Un asset debe poder arrastrarse desde Library hacia cualquier feature compatible.

No duplicar archivos si puede referenciarse el mismo asset.

---

## 16. Queue

La cola pertenece al core de VideoGeneratorAI, no a ComfyUI ni a otro backend.

Estados base:

```text
Queued
Preparing
Downloading
Loading
Encoding
Sampling
Decoding
PostProcessing
Completed
Failed
Cancelled
```

La UI debe mostrar:

- engine;
- modelo;
- etapa;
- progreso;
- tiempo;
- VRAM;
- logs resumidos;
- cancelación.

---

## 17. Pipelines

La UI puede exponer pipelines encadenables sin convertirse en un editor de nodos.

Ejemplos:

```text
Generate
   ↓
Enhance 1080p
   ↓
Foley
   ↓
Export
```

```text
OmniWeaving Edit
   ↓
SR
   ↓
Foley
```

Cada bloque corresponde internamente a un job independiente.

Esto mantiene modularidad sin obligar al usuario a trabajar con grafos complejos.

---

## 18. Hardware como parte de la UI

La aplicación debe detectar y representar:

- GPU;
- VRAM;
- arquitectura CUDA;
- driver;
- RAM;
- swap;
- almacenamiento relevante.

Ejemplo:

```text
RTX 3090 · 24 GB

✓ BF16
✓ INT8
⚠ FP8 no nativo
```

Los presets deben adaptarse al hardware.

No mostrar configuraciones inviables como si fueran equivalentes.

---

## 19. Presets

Presets de alto nivel:

- Preview;
- Balanced;
- Quality;
- Custom.

Los valores no deben hardcodearse globalmente.

Cada engine/modelo declara cómo interpreta cada perfil.

Ejemplo conceptual:

```text
Preview
→ baja resolución
→ modelo distilled
→ máxima velocidad

Balanced
→ calidad/velocidad equilibradas

Quality
→ resolución alta
→ pasos completos
→ SR / postprocesado
```

---

## 20. Estado y lógica

Los widgets no ejecutan lógica de negocio.

Un componente debe:

- recibir estado;
- dibujar;
- devolver intención/eventos.

No debe:

- abrir DB;
- lanzar Python;
- tocar filesystem;
- controlar directamente engines;
- contener estado global.

Ejemplo conceptual:

```text
Button
 ↓
UI Event
 ↓
Application Command
 ↓
Use Case
 ↓
Engine / Storage / Service
```

---

## 21. Accesibilidad

Desde el inicio:

- navegación por teclado;
- focus visible;
- labels accesibles;
- contraste;
- tamaño de targets razonable;
- estados no expresados sólo mediante color;
- shortcuts documentados.

No dejar accesibilidad para una fase final.

---

## 22. Rendimiento

- UI thread nunca bloqueado por inferencia o I/O pesado;
- thumbnails cacheados;
- decodificación de video fuera del frame crítico;
- virtualización de listas largas;
- repaints sólo cuando hagan falta;
- previews progresivos;
- telemetría sin polling agresivo;
- medir antes de optimizar.

---

## 23. Reutilización futura

El UI kit debe poder usarse en otras apps.

Pero la extracción a un repositorio/librería independiente debe hacerse sólo cuando aparezca una segunda aplicación que lo consuma.

Proceso:

```text
VideoGeneratorAI/ui-kit
        ↓
segunda app lo necesita
        ↓
extraer API estable
        ↓
ui-kit compartido
```

No crear una plataforma abstracta antes de necesitarla.

---

## 24. Regla final

La UI debe quedar organizada así:

```text
egui / eframe
      ↓
componentes externos maduros cuando convenga
      ↓
nuestro UI kit
      ↓
features de VideoGeneratorAI
      ↓
commands
      ↓
application/core
```

El objetivo es que podamos cambiar:

- theme;
- widgets;
- docking;
- iconografía;
- backend gráfico;

sin reescribir la lógica de negocio ni los motores.
