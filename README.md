# rwm

## Dependencies:

You will need some kind of rust compiler, xorg-server, xcb-util-cursor and pango though you probably have that installed already.

`sudo pacman -S xorg-server xcb-util-cursor pango`

## Configuration

You will find all the configuration inside the [config.rs](/src/config.rs)

## Installation:

```
make

sudo make install

make clean
```

## Usage:

If you use rwm with startx you will need xorg-xinit

`sudo pacman -S xorg-xinit`

and put following line at the enf of your ~/.xinitrc.

`exec dbus-launch --exit-with-session rwm`

## Status

The status represents the roots WM_NAME which you can easily set with xsetroot.

`sudo pacman -S xorg-xsetroot`

An easy status could look as followed

```sh
#!/bin/sh

while true; do
	xsetroot -name "$(date +"%a %d/%m/%Y %T")"
	sleep 1
done
```

Or you could use my [rstatus](https://github.com/JuliusKreutz/rstatus)
