# LAPUTA-001-A — User can create a project and safely import mixed media

Parent: LAPUTA-001  
Type: Product Story  
Status: Ready  
Dependencies: None  
Decision links: `docs/decisions/ADR-0003-agent-first-consumer-interaction.md`

As a person making a personal video,  
I want to create a project and add local photos/videos without worrying about damaging them,  
so that I can start creating while my originals remain safe.

## Scope

- Project identity/location and persisted minimal project metadata.
- Import by file picker and/or drag-and-drop on the desktop shell.
- Asset records referencing originals; basic probe metadata and project-local derivative/cache locations.
- Original immutability contract and hash-based test fixture evidence.
- Project reopen with imported asset inventory restored.

## Out of scope

- AI analysis, generation, first cut, cloud sync, account system, mobile import.

## Acceptance

- Given a new Laputa project  
  When the user imports a fixture containing supported photos and videos  
  Then the asset browser shows imported items  
  And original files remain byte-for-byte unchanged.
- Given an imported project  
  When Laputa creates metadata/proxies/thumbnails  
  Then derivatives are stored separately from originals.
- Given the application is closed after import  
  When the same project is reopened  
  Then the imported asset inventory can be reconstructed/restored without re-importing originals.

## Minimum validation

- Automated fixture test records original hashes before/after import and derivative generation.
- Rust workspace checks/tests for affected crates.
- Manual desktop smoke check for import and reopen when GUI path exists.

## State owners to synchronize

- `docs/backlog/PRODUCT-BACKLOG.md`
- parent LAPUTA-001 summary
- active iteration record
- user-facing README if observable startup/import behavior changes
