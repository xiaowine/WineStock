# Implementation Notes

This directory stores implementation plans, design notes, and historical decision drafts.

Files here are non-normative by default. They do not add standing agent constraints unless a normative project document explicitly references them or a user asks to use one for a task.

## Current notes

- `business-api-implementation-plan.md`: detailed implementation plan for the first stock business API slice described by `docs/business-api.md`.
- `core-axum-structure-refactor-plan.md`: domain-oriented refactor plan for `core\src` API growth.
- `core-spring-boot-style-refactor-plan.md`: historical implementation record for the refactor that moved `core\src` to `http / security / auth / users / rbac` with `controller + service` modules.
- `stock-search-filter-values-plan.md`: design and implementation constraints for stock item, inbound, and outbound search/filter-values APIs.
