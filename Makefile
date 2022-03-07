PROJECT=hconnect
PANDOC?=pandoc
PANDOCFLAGS?=--from markdown+tex_math_dollars --mathjax --to html5 --standalone --metadata pagetitle="${PROJECT}"

all:
	cargo clippy
	cargo fmt

%.html: %.md
	${PANDOC} ${PANDOCFLAGS} -o $@ $<

.PHONY: all
