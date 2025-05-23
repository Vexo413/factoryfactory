# Factory Factory

## Description

This is my factory game about collecting resources, processing them, and then combining them into more factory parts, and the cycle continues.

I first saw factory games on youtube like Satisfactory, Factorio and Shapez.io. I always wanted to play them, but I was too poor to get any that interested me. What eventually made me begin to make this game was Between Bytes video about his factory game, [Frantic Factories](https://youtu.be/GkWXgl4rVo0?si=7NJ-KN14OuzUYOl5). I came up with the theme a factory for expanding that same factory, and started coding it.

## Updates

So far the base factory engine is completed, the next step is to add ui to the placing mechanics.

## Build

First make sure you have rust installed.

[1]: https://www.rust-lang.org/learn/get-started

Download rust [here][1].

To get the files, either use git,

```
git clone https://github.com/Vexo413/factoryfactory.git
```

[2]: https://github.com/Vexo413/factoryfactory/releases/tag/v0.1.0

Or download the zip file from the github page [here][2].

Then go to the project directory in your terminal and run:

```
cargo run --release
```

## Controls

`WASD`: Move camera

`E`: Inventory / Tile selection

`Scroll`: Cycle through tiles / Zoom

`Left Click`: Place selected tile / Core menu

`Right Click`: Remove tile


## Links

[3]: https://www.rust-lang.org/

- [Rust][3]

[4]: https://bevyengine.org/

- [Bevy][4]
