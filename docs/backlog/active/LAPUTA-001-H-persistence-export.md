# LAPUTA-001-H — User can reopen the project and export the integrated result

Parent: LAPUTA-001  
Type: Product Story  
Status: Proposed  
Dependencies: integration closure across required LAPUTA-001 children  
Decision links: `docs/reference/PRODUCT-REQUIREMENTS-v0.1.md`

As a user, I want to close and reopen my project and export the current result, so that the work survives a session and becomes a shareable video.

## Scope

- Restore latest committed intent, imported asset inventory, timeline, locks and derived-artifact lineage required to continue editing.
- Final deterministic render/encode to MP4.
- 16:9 and 9:16 supported output.
- End-to-end fixture acceptance from import through revision and export.

## Out of scope

- Cloud sync, social publishing, collaboration, release packaging of the desktop app itself.

## Acceptance

- Given a project with first cut and revision  
  When Laputa is closed and reopened  
  Then the user can continue from the latest committed project state.
- Given a valid current video  
  When export is requested  
  Then a playable MP4 is produced in a supported aspect ratio  
  And original fixture hashes remain unchanged.
- Given the integrated MVP scenario  
  When executed from import through one selection-based revision to export  
  Then no normal interaction requires provider/model/workflow selection.

## Minimum validation

End-to-end fixture test, output probe/playability check, project reopen check, and original-media hash evidence.
