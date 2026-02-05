# MatchupHelper

Aplicación de escritorio ultra-ligera para gestionar notas personales de matchups de League of Legends.

## Características

- **CRUD de Matchups**: Crea, edita y elimina matchups entre campeones
- **Sistema de Versionado**: Cada edición crea una nueva versión automáticamente
- **Tags**: Categoriza matchups con tags como "easy", "hard", "early-game", etc.
- **Búsqueda y Filtros**: Encuentra matchups por campeón, rol o texto en notas
- **Iconos de Data Dragon**: Muestra iconos de campeones directamente desde el CDN de Riot
- **Integración LCU**: Conecta al cliente de LoL para importar historial de partidas
- **Always-on-top**: Mantén la ventana siempre visible mientras juegas

## Stack Técnico

- **Backend**: Rust + Tauri v2
- **Frontend**: HTML/CSS/JS vanilla
- **Almacenamiento**: JSON en disco (`%APPDATA%/matchuphelper/`)
- **Assets**: Data Dragon CDN de Riot

## Requisitos

- Windows 10/11
- [Rust](https://rustup.rs/) instalado
- [Node.js](https://nodejs.org/) v18+
- Visual Studio Build Tools (para compilar en Windows)

## Instalación

```bash
# Clonar repositorio
git clone https://github.com/javiomaster211/matchup_helper.git
cd matchup_helper

# Instalar dependencias
npm install

# Ejecutar en desarrollo
npm run tauri dev

# Compilar para producción
npm run tauri build
```

## Estructura del Proyecto

```
matchuphelper/
├── src-tauri/           # Backend Rust
│   ├── src/
│   │   ├── main.rs      # Entry point
│   │   ├── lib.rs       # Comandos Tauri
│   │   ├── matchup.rs   # Lógica de matchups
│   │   ├── storage.rs   # Persistencia JSON
│   │   └── lcu.rs       # Conexión al cliente LoL
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                 # Frontend
│   ├── index.html
│   ├── styles.css
│   └── app.js
└── package.json
```

## Uso

1. Abre la aplicación
2. Crea un nuevo matchup con el botón "+ Nuevo"
3. Selecciona tu campeón, el enemigo y el rol
4. Añade notas sobre el matchup
5. Usa tags para categorizar (easy, hard, early-game, etc.)
6. Busca matchups existentes con el buscador

### Atajos de Teclado

- `Ctrl+Shift+M`: Enfocar búsqueda
- `Escape`: Cerrar modales

## Modelo de Datos

Los datos se guardan en `%APPDATA%/matchuphelper/data.json`:

```json
{
  "matchups": {
    "uuid-123": {
      "id": "uuid-123",
      "my_champion": "Darius",
      "enemy_champion": "Garen",
      "role": "top",
      "versions": [...],
      "current_version": 2
    }
  }
}
```

## Licencia

MIT
