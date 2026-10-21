# Stack y toolbox

Este documento define el **toolbox preferido** de VideoGeneratorAI y las reglas para elegir tecnologías.

No significa que todas deban utilizarse al mismo tiempo. Cada módulo debe usar **el conjunto mínimo que resuelva bien su responsabilidad**.

---

## 1. Principios de selección

- **Rust por defecto** para el core, lógica de aplicación, integración con el sistema y componentes nativos.
- **egui / eframe** como primera opción para UI nativa.
- **wgpu** cuando necesitemos renderizado GPU propio.
- **TypeScript** sólo cuando una interfaz web o WebView aporte una ventaja concreta.
- **Python** sólo donde el ecosistema de IA/ML lo requiera.
- No introducir una tecnología únicamente porque esté disponible.
- No duplicar responsabilidades entre lenguajes o frameworks.
- Toda dependencia externa importante debe quedar detrás de una interfaz propia.

---

## 2. Rust

Rust es el lenguaje principal del proyecto.

Responsabilidades preferidas:

- dominio;
- lógica de aplicación;
- orquestación;
- job scheduler;
- engine registry;
- filesystem;
- persistencia;
- configuración;
- networking;
- procesos y subprocesses;
- hardware/GPU discovery;
- telemetría local;
- integración con el sistema operativo;
- UI nativa;
- protocolo con motores externos.

La lógica crítica debe permanecer independiente de la UI y de implementaciones concretas de engines.

---

## 3. egui / eframe

Primera opción para aplicaciones de escritorio nativas.

Usarlo para:

- shell de escritorio;
- paneles;
- inspectors;
- timeline;
- drag & drop;
- previews;
- biblioteca multimedia;
- cola de trabajos;
- model manager;
- configuración;
- herramientas de edición;
- overlays y controles interactivos.

La aplicación no debe llenar el código con llamadas directas a widgets externos. Debe consumir nuestro propio **UI kit**.

Arquitectura objetivo:

```text
egui / eframe
      ↑
dependencias visuales externas opcionales
      ↑
VideoGeneratorAI UI Kit
      ↑
features de la aplicación
```

---

## 4. wgpu

Usar `wgpu` cuando haga falta renderizado GPU propio.

Casos:

- video preview avanzado;
- composición;
- canvas;
- overlays;
- shaders;
- procesamiento 2D/3D;
- visualizaciones;
- renderizado de timeline o thumbnails cuando egui no sea suficiente.

Regla:

> Si eframe/egui ya resuelve el caso correctamente, no bajar a wgpu directo sin una necesidad medida.

---

## 5. TypeScript

TypeScript forma parte del toolbox, pero **no es obligatorio en VideoGeneratorAI**.

Usarlo si en el futuro necesitamos:

- frontend web;
- interfaz remota;
- Tauri/WebView;
- extensiones web;
- paneles embebidos;
- herramientas donde HTML/CSS aporten una ventaja clara.

No introducir React/TypeScript sólo por costumbre si la UI nativa Rust cubre mejor el caso.

---

## 6. Python

Python se reserva principalmente para motores de IA y ecosistemas cuyo código principal dependa de PyTorch u otras librerías Python.

Ejemplos:

- HunyuanVideo;
- LightX2V;
- OmniWeaving;
- HunyuanVideo-Avatar;
- HunyuanVideo-Foley;
- trainers;
- pipelines de terceros.

Python **no forma parte del core**.

Arquitectura obligatoria:

```text
VideoGeneratorAI
      ↓
Engine interface
      ↓
Adapter / bridge
      ↓
Proceso Python aislado
      ↓
PyTorch / modelo / runtime
```

Evitar:

```text
UI
 ↓
scripts Python arbitrarios
 ↓
estado compartido
```

---

## 7. Aislamiento de engines

Cada engine externo debe poder tener:

- entorno Python independiente;
- versión propia;
- dependencias propias;
- commit/version fijado;
- lifecycle independiente;
- logs propios;
- health check;
- cancelación;
- timeout;
- reinicio.

Ejemplo:

```text
engines/
├── hunyuan15-lightx2v/
├── omniweaving/
├── hunyuan-avatar/
├── hunyuan-foley/
└── comfyui/
```

Actualizar un engine no debe romper los demás.

---

## 8. Protocolo de engines

El core no debe conocer detalles internos de Hunyuan, ComfyUI o cualquier otro backend.

Capacidades conceptuales:

```text
probe
describe
prepare
submit
cancel
unload
health
estimate
```

Eventos:

```text
started
loading_model
encoding
sampling
decoding
progress
preview
warning
completed
failed
```

Opciones de transporte aceptables:

- JSONL por stdin/stdout;
- JSON-RPC;
- HTTP local;
- gRPC cuando realmente se justifique.

El protocolo debe ser explícito y versionado.

---

## 9. Arquitectura por capas

Separación base:

```text
UI
↓
Application
↓
Domain
↓
Infrastructure / Adapters
```

### Domain

- tipos principales;
- invariantes;
- reglas de negocio.

No depende de UI, filesystem, red, DB ni engines.

### Application

- casos de uso;
- commands;
- events;
- orquestación.

### Infrastructure / Adapters

- SQLite;
- filesystem;
- procesos;
- red;
- GPU;
- engines externos;
- APIs.

### UI

- presentación;
- estado efímero;
- interacción;
- traducción de acciones del usuario a comandos.

La UI no accede directamente a motores, shell, DB ni filesystem.

---

## 10. Modularidad

Objetivo:

> Cambiar, romper o reemplazar un módulo no debe afectar módulos no relacionados.

Reglas:

- alta cohesión;
- bajo acoplamiento;
- interfaces pequeñas;
- dependencias explícitas;
- componentes reemplazables;
- fallos aislados;
- evitar estado global mutable;
- una fuente de verdad por dato;
- evitar módulos genéricos tipo `utils`, `helpers`, `misc` o `common` como cajones de sastre.

---

## 11. Core estable

Principio central:

```text
Core estable
     ↓
interfaces estables
     ↓
implementaciones reemplazables
```

Ejemplo:

```text
VideoEngine
├── Hunyuan 1.5 + LightX2V
├── OmniWeaving
├── ComfyUI
├── WanGP opcional
└── MockEngine
```

El proyecto nunca debe quedar atado a un solo runtime.

---

## 12. Concurrencia

- Nunca bloquear el thread de UI.
- Trabajo pesado fuera de UI.
- Jobs cancelables.
- Progreso observable.
- Shutdown limpio.
- Timeouts.
- Control de procesos huérfanos.
- Async sólo donde aporte valor real.

---

## 13. Persistencia

Preferencias y configuración:

- archivos versionados y migrables.

Datos estructurados:

- SQLite.

Archivos grandes:

- filesystem.

La DB guarda metadata y referencias; no blobs grandes salvo necesidad concreta.

---

## 14. Logging

Preferencia Rust:

- `tracing`.

Separar:

- trace;
- debug;
- info;
- warn;
- error.

Los engines externos deben identificarse de forma separada en logs.

---

## 15. Testing

- unit tests para domain;
- integration tests para adapters;
- contract tests para engines;
- protocol tests;
- persistence tests;
- UI behavior tests cuando tengan valor;
- snapshots/golden tests cuando correspondan;
- MockEngine para validar el ciclo completo antes de conectar modelos reales.

Flujo mínimo del MockEngine:

```text
submit
progress
preview
cancel
complete
fail
restart
```

---

## 16. Seguridad

- mínimo privilegio;
- UI sin shell arbitrario;
- paths validados;
- inputs externos validados;
- ejecutar procesos con argumentos, no strings de shell;
- motores y plugins tratados como componentes externos;
- permisos mínimos;
- no ejecutar código remoto implícitamente.

---

## 17. Reproducibilidad

Para cada trabajo relevante registrar:

- versión de VideoGeneratorAI;
- engine;
- versión/commit del engine;
- modelo;
- versión del modelo;
- inputs;
- prompt;
- parámetros;
- seed;
- hardware relevante;
- tiempos;
- resultado.

Especialmente obligatorio en workflows de IA.

---

## 18. CI base

Como mínimo:

```text
cargo fmt
cargo clippy
cargo test
cargo build
```

Agregar según el proyecto:

- integration tests;
- UI tests;
- auditoría de dependencias;
- verificación de licencias;
- builds Linux/Windows/macOS;
- benchmarks;
- contract tests.

---

## 19. Regla final

El stack es un toolbox, no una receta fija.

```text
Rust
egui / eframe
wgpu
TypeScript
Python
```

La tecnología se elige por necesidad.

Los principios que no cambian son:

```text
modularidad
+
separación de responsabilidades
+
bajo acoplamiento
+
interfaces explícitas
+
componentes reemplazables
+
fallos aislados
```
