# pakk

Kiire ja tõhus mitmelõimeline pakkimistööriist — nagu `zip` või `tar`, aga
moodsate algoritmidega (zstd ja xz), mis on ühes väikeses staatilises binaaris.

**Miks parem kui zip/gzip?** Sama korpuse peal on `pakk --fast` gzip'ist
**7× kiirem** ja teeb **väiksema** faili; vaikimisi režiim pakib **kolmandiku
võrra tihedamalt** kui gzip ja `--max` veel tihedamalt (vt mõõtmisi allpool).

Arhiivivorming on standardne tar-voog zstd või xz pakendis, seega failid
avanevad ka igal pool mujal: `tar --zstd -xf fail.pakk` või `tar -Jxf`.

## Kasutamine

```sh
pakk c projekt.pakk src/ docs/         # loo arhiiv (zstd-19 + long, tasakaalus)
pakk c --fast logid.pakk /var/log      # kiireim (zstd-3, mitmelõimeline)
pakk c --max varukoopia.pakk andmed/   # tihedaim (xz-9 extreme)
pakk c -l 22 suur.pakk andmed/         # käsitsi zstd tase 1–22
pakk x projekt.pakk -C /tmp/valjund    # paki lahti
pakk l projekt.pakk                    # näita sisu ilma lahti pakkimata
```

Režiimid:

| Režiim    | Algoritm            | Millal kasutada                          |
|-----------|---------------------|------------------------------------------|
| `--fast`  | zstd-3, kõik tuumad | igapäevane kiire pakkimine, logid, ajutised failid |
| vaikimisi | zstd-19 + long-distance matching | hea suhe/kiirus — enamik olukordi |
| `--max`   | xz-9e, kõik tuumad  | varukoopiad ja levitamine, kus loeb iga bait |

Lahtipakkimisel tuvastatakse vorming automaatselt maagiliste baitide järgi —
lippe pole vaja. Sümbolviited, õigused ja ajatemplid säilivad.

## Mõõtmised

Korpus: Go standardteegi lähtekoodipuu, 107,6 MiB, 9000+ faili.
Masin: 4 tuuma, Linux x86_64. Kõik käsud jooksid samas keskkonnas.

| Tööriist            | Suurus   | % algsest | Aeg    |
|---------------------|----------|-----------|--------|
| `pakk --fast`       | 24,2 MiB | 22,5 %    | 0,7 s  |
| `pakk` (vaikimisi)  | 18,3 MiB | 17,0 %    | 29,6 s |
| `pakk --max`        | **17,4 MiB** | **16,2 %** | 92,3 s |
| `zip -r`            | 32,5 MiB | 30,2 %    | 4 s    |
| `tar czf` (gzip)    | 27,9 MiB | 25,9 %    | 5 s    |
| `tar cJf` (xz-6)    | 18,3 MiB | 17,0 %    | 43 s   |

## Paigaldamine

Lähtekoodist (vajab [Rusti](https://rustup.rs)):

```sh
cargo install --path .
# või ilma kloonimata:
cargo install --git https://github.com/nagistaja/pakk
```

Valmis binaarid: iga `v*`-sildiga release'i juures on staatiline Linux-binaar
tar-pallina (`pakk-vX.Y.Z-linux-x86_64.tar.gz`) — laadi alla, paki lahti ja
pane `PATH`-i.

## Arhitektuur

- **Konteiner:** POSIX tar (`tar` crate) — säilitab kataloogipuu, õigused,
  sümbolviited ja ajatemplid.
- **zstd** (`libzstd`, staatiliselt lingitud): mitmelõimeline pakkimine,
  long-distance matching 128 MiB aknaga (`--long=27`-ühilduv), tasemed 1–22.
- **xz** (`liblzma`, staatiliselt lingitud): mitmelõimeline `-9e`-kodeerija
  maksimaalse pakkimissuhte jaoks.
- Lahtipakkimine tuvastab vormingu maagiliste baitide järgi
  (`28 B5 2F FD` = zstd, `FD 37 7A 58 5A 00` = xz).
- Turvalisus: lahtipakkimine käib `unpack_in` kaudu, mis blokeerib
  path-traversal-rünnakud (`../../etc/passwd`-stiilis kirjed).

## Arendus

```sh
cargo test            # ühik- ja integratsioonitestid
cargo build --release # optimeeritud binaar target/release/pakk
```

## Litsents

MIT
