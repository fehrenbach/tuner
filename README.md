[![Build Status](https://travis-ci.org/fehrenbach/tuner.svg?branch=master)](https://travis-ci.org/fehrenbach/tuner)

ARM Cortex M3
=============

Compile with (-g debugging, optionally use optimisation)
> $ arm-none-eabi-gcc -std=c99 autocorrelation.c -mcpu=cortex-m3 -mthumb -T generic-hosted.ld -g

Run in qemu with
> $ qemu-arm -cpu cortex-m3 a.out

Samples
=======

Record sample with
> $ arecord -traw -fS8 -r32768 -c1 > bass-$SOMEHz-fS8-r32768

Play sample bass-41.2Hz-fS8-r32768 with
> $ aplay -fS8 -r32768 bass-41.2Hz-fS8-r32768

Math
====

Frequencies
-----------

with A4 at 440Hz

C1 32.7032 Hz
E1 41.2034 Hz (Bass E)
A1 55 Hz
C2 65.4064 Hz


Phase shifts
------------

at 32768 Hz
phase frequency
594   55.164982
595   55.072269
596   54.979866

at 44.1 kHz
phase frequency
799   55.194
800   55.125
801   55.056
802   54.988
803   54.919


Equations
---------

sr - sampling rate
f  - frequency
p  - phase shift

f = 1/(p/sr)

p = sr/f
