# OxSchnick

A distributed Rock-Paper-Scissors game for students at the University of Oxford!

Forked from [Fanschnick](https://codeberg.org/fanschnick/fanschnick-server/) developed by Helena Jäger et al. at the University of Hamburg's [informatics society](https://mafiasi.de/dashboard/) and first played at [39C3](https://events.ccc.de/congress/2025/infos/startpage.html).
Many thanks for the development work!

## Building

Copy `.env.example` to `.env` and adapt as necessary.
**Note:** the `DATABASE_URL` environment variable is ignored (and reset) if building through Docker Compose.
Install docker and use `docker compose up` and `docker compose down` to build and destroy.
