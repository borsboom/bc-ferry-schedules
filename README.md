# B.C. Ferry Schedules for the Outer Gulf Islands

An easy to use and understand presentation of the BC Ferries schedules serving
Galiano, Mayne, Pender, and Saturna Islands to/from Victoria, Vancouver, and
Salt Spring Island. Just select your terminals and date, and you're shown the
sailings for that day.

Web site: https://ferries.borsboom.io/

This repository contains a scraper to download and parse BC Ferries schedules,
and a single page web app front-end to browse the sailings.

## Build and run locally

 1. Install required build tools:

      * [Rust toolchain](https://www.rust-lang.org/tools/install) - to build
        the scraper and front-end, which are written in Rust.
      * [Trunk](https://trunkrs.dev/#install) - to build and view the
        front-end.
      * [just](https://github.com/casey/just#installation) - to use commands in
        the [Justfile](Justfile).

 2. Build and run scraper to generate local schedule data file:

        just local-data

 3. Build and serve the front-end web app:

        just local-frontend

 4. Open http://localhost:8080/ to view the front-end web app.
## License

Copyright © 2022 Emanuel Borsboom.

Licensed under either of

  * Apache License, Version 2.0 ([LICENSE-APACHE.txt](LICENSE-APACHE.txt) or
    http://www.apache.org/licenses/LICENSE-2.0)
  * MIT license ([LICENSE-MIT.txt](LICENSE-MIT.txt) or
    http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
