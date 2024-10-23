# Processor Affinity Optimizer
IDK what to call it, ok?

# How To Use
Inject this DLL into your game, and make sure that there is a process. This mod was made for Elden Ring, but should work for any
process that loads dinput8, or you inject into some other way. This should help with performance for processes that are running on all cores,
including the core your OS uses for most tasks.

## dinput8.dll
You can extract the `dinput8.dll` and `affinity.toml` to the game folder, and it will be loaded into the game when you 
launch it. For Elden Ring, you HAVE to run the game with EAC turned off, somehow. I recommend just using `steam_appid.txt` 
with the app id `1245620` inside of it, and then launching the game from `eldenring.exe` instead of steam.

## Dll injector
If you are using something like lazy loader, elden mod loader, or modengine2, then you can rename the `dinput8.dll` to anything
and load up the dll that way. Make sure that the `affinity.toml` is also placed alongside the dll, as it reads it from the
directory that it's in.

# Config

## Delay
This is a delay before the dll changes your core affinity. Setting this too soon may cause your game to crash. Ideally you
want to find a timing that allows it to load before you get to the main menu.

## Exclude
An array of cores to exclude. By default, all cores are included. This program is set up to exclude core 0, but you can
customize it. See the toml file for details. You can find the cores avaiible on your processor by going into the task manager, 
details panel, right click on any process and choose "set affinity".


# Thanks
Thank you to [KUPOkinz](https://www.youtube.com/@kupokinzyt) for bringing this to my attention in this video. https://www.youtube.com/watch?v=76Wl4KKmEs8 
