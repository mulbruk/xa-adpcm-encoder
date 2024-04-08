XA ADPCM Encoder
================

Quick and dirty tool for encoding WAV files to the format used by Riverhillsoft in their Sega Saturn games. (Other Saturn games may use the same format, idk).

The encoding used by Riverhillsoft consists of raw 2324-byte CD-ROM XA audio blocks packed into an AIFF container. This software implements the encoding algorithm defined in the [CD-ROM XA Specification](https://archive.org/details/xa-10-may-1991) (Sony / Philips, 1991).

This was written late at night and the pieces are kind of hacked together, might finish cleaning it up sometime.
