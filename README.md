ARM Cortex M3
-------------

Compile with (-g debugging, optionally use optimisation)
> $ arm-none-eabi-gcc -std=c99 autocorrelation.c -mcpu=cortex-m3 -mthumb -T generic-hosted.ld -g

Run in qemu with
> $ qemu-arm -cpu cortex-m3 a.out

Samples
-------

Record sample with
> $ arecord -traw -fS8 -r32768 -c1 > bass-$SOMEHz-fS8-r32768

Play sample bass-41.2Hz-fS8-r32768 with
> $ aplay -fS8 -r32768 bass-41.2Hz-fS8-r32768
