A small proxy for kotlin-lsp to Helix, because Helix doesn't support JAR paths.

The proxy spawns an instance of kotlin-lsp and forwards messages to Helix after
rewriting the JAR:// path to a file:// path, unzipping the file to the default
caching destination of the OS.

Tested on MacOS.
