# larp-pdf

Runs the `tectonic` program on `.tex` sources produced by `larp-render`.

- The only library crate that spawns processes.
- Reports missing `tectonic` with an install hint ([`PdfError::Missing`]) instead of a raw OS error.

```no_run
let done = larp_pdf::compile(std::path::Path::new("dist/pdf/anselm.tex"))?;
println!("wrote {}", done.pdf.display());
# Ok::<(), larp_pdf::PdfError>(())
```
