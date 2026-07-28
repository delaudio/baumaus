# Baumaus

Baumaus is a small, script-first architectural CAD that lives entirely in the terminal. It is an original Ratatui implementation: edit a plan on the left and see a live 3D model on the right, rendered inline with the Ratty Graphics Protocol.

```sh
ratty -e cargo run
```

Controls: `Tab` switches panes. In the 3D pane, arrows rotate, `z`/`x` zoom, and `r` resets the view. `F5` builds, `a` toggles automatic build, `s` saves `baumaus.json`, and `q` quits. The editor accepts `wall`, `door`, and `window` calls using millimetres:

```text
wall([0, 0], [6000, 0], thickness = 300);
door("wall-001", offset = 1200, width = 900);
window("wall-002", offset = 900, width = 1200, sill = 900);
```

The model is also saved as structured JSON with `s`; it is the source of truth, while the Ratty OBJ preview is derived on every successful build.

## Attribution

The Ratty preview integration follows the public API usage pattern of [ratSCAD](https://github.com/qewer33/ratscad), licensed under MIT by qewer33. Baumaus does not include ratSCAD source code.
