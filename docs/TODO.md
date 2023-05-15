# TODO
## Quine Protection
Quines are archives that extract themselves, so if you try to recursievly extract *all* the files from them, it will run until you crash.

For extraction a Directed Acyclic Graph should be maintained so that extraction can be forced to stop when a cycle would be created.
## Cross Compilation
Cross compiling with `extract` enabled currently fails when trying to find libarchive.