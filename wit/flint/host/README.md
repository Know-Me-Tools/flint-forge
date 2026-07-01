# flint:host@0.1.0

The capability contract every edge component implements. A component EXPORTS
`wasi:http/incoming-handler` and may IMPORT the governed host interfaces it declares in
its signed manifest. Granted capabilities = manifest.capabilities ∩ Cedar(publisher).

`json` payloads are modeled as `string` here for WIT portability; the host (de)serializes.
