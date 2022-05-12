# GUI

The Screen object contains the back buffer of a double-buffered screen. The front buffer of the screen is the graphics system's framebuffer.

Each screen consists of one or more Windows. Each Window represents a subsection of the screen, possibly overlapping, defined by an (x,y) offset from the top left corner, and a (w,h) size. Each application that has a Window can freely write to it whenever it pleases (there is no vsync implemented at this time).

When the Screen is ready to draw a new frame, it first copies the contents of each Window's buffer to the Screen's back buffer, and then copies the Screen's back buffer to the front buffer.

All buffers are contiguous, in RGBA (4 bytes) order, and stride equal to the horizontal resolution.