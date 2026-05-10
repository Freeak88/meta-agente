# Public Release Checklist v1.0

## Código

- [x] Tests verdes (95 passed)
- [x] `cargo fmt --check` OK
- [ ] `cargo clippy` limpio (pendiente de correr)
- [x] Sin secrets hardcodeados
- [x] Sin credenciales en repo
- [x] `.gitignore` correcto (`target/`, `snapshots/`, `*.pdb`, `.env`)

## Documentación

- [x] `README.md` actualizado a v1.0
- [x] Quickstart: `cargo run -- --server`
- [x] Arquitectura explicada para humanos
- [x] Changelog v0.9 a v1.0
- [x] `CONTRIBUTING.md` / flujo de contribución documentado en repo
- [x] `LICENSE` MIT

## Seguridad básica

- [x] Sin API keys en código
- [x] Sin contraseñas en tests
- [x] Config por environment variables documentada como próximo paso
- [x] Snapshots locales no se pushean

## Landing page

- [x] Problema entendible en segundos
- [x] Demo de `cargo run -- --server`
- [x] Arquitectura simple
- [x] Roadmap: v1.1 MCP, v1.2 Cloud, v1.3 Enterprise
- [x] CTA GitHub
- [ ] Newsletter / Discord (opcional)

## Post-release

- [ ] GitHub Settings -> General -> Visibility: Change to Public
- [ ] GitHub Pages: Enable, source `main` / `landing` folder
- [ ] Social preview: Upload image (logo + tagline)
- [ ] Topics: `rust`, `agents`, `automation`, `dsl`, `mcp`
- [ ] Hacker News post
- [ ] Reddit `r/rust`, `r/programming`
- [ ] LinkedIn post técnico
- [ ] Buscar 3 beta testers reales
