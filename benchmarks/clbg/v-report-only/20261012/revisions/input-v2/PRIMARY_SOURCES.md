This revision uses the already selected official V0.5.2 source commit
`7647ce1c6fad63b5578bc07883139906de74b2f8`.

- [io.read_all / ReadAllConfig](https://raw.githubusercontent.com/vlang/v/7647ce1c6fad63b5578bc07883139906de74b2f8/vlib/io/reader.v): standard Reader loop, returned byte buffer, explicit read-to-end option.
- [os.stdin / File.read](https://raw.githubusercontent.com/vlang/v/7647ce1c6fad63b5578bc07883139906de74b2f8/vlib/os/file.c.v): descriptor0, fread chunks and EOF reporting.
- Original published ancestry and prior source paths are preserved per profile
  in `history/parent-profiles.json`; the root-owned stopped first qualification
  remains under `qualification/production-sdk-v1` in the V comparison root.

No compiler, native probe or hash sweep was performed during text authoring.
