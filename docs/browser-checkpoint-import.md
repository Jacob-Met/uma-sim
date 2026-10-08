# Import an exported career checkpoint

In the Career library, choose a checkpoint JSON file using the existing Import control. Leave the optional name blank to use the native simulator's generated name, or enter a new library name.

Import preserves the complete snapshot JSON text, including signed 64-bit seeds that cannot be represented exactly by JavaScript numbers. The native simulator still checks the checkpoint's structure, compatibility and library name before storing it. Import does not overwrite an existing named checkpoint implicitly.

Malformed JSON and JSON values other than objects are rejected before a request is sent. Native validation failures continue to appear through the existing library error display. After a successful import the library list refreshes; use the existing Resume action to continue the imported career.
