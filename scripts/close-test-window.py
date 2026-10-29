"""Send the real window-manager close event in the private Xvfb test session."""
import ctypes as c
import os
from pathlib import Path
import sys

assert Path('/.dockerenv').exists() and os.environ.get('COMPUTY_TEST_X11') == '1'
pid = int(sys.argv[1])
x = c.CDLL('libX11.so.6')
x.XOpenDisplay.argtypes = [c.c_char_p]
x.XOpenDisplay.restype = c.c_void_p
x.XDefaultRootWindow.argtypes = [c.c_void_p]
x.XDefaultRootWindow.restype = c.c_ulong
x.XInternAtom.argtypes = [c.c_void_p, c.c_char_p, c.c_int]
x.XInternAtom.restype = c.c_ulong
x.XQueryTree.argtypes = [
    c.c_void_p, c.c_ulong, c.POINTER(c.c_ulong), c.POINTER(c.c_ulong),
    c.POINTER(c.POINTER(c.c_ulong)), c.POINTER(c.c_uint),
]
x.XGetWindowProperty.argtypes = [
    c.c_void_p, c.c_ulong, c.c_ulong, c.c_long, c.c_long, c.c_int, c.c_ulong,
    c.POINTER(c.c_ulong), c.POINTER(c.c_int), c.POINTER(c.c_ulong),
    c.POINTER(c.c_ulong), c.POINTER(c.POINTER(c.c_ubyte)),
]
x.XFree.argtypes = [c.c_void_p]
x.XCloseDisplay.argtypes = [c.c_void_p]
display = x.XOpenDisplay(None)
assert display


def prop(window, key):
    atom = x.XInternAtom(display, key.encode(), 0)
    kind, fmt, count, remaining = c.c_ulong(), c.c_int(), c.c_ulong(), c.c_ulong()
    data = c.POINTER(c.c_ubyte)()
    result = x.XGetWindowProperty(
        display, window, atom, 0, 1024, 0, 0, c.byref(kind), c.byref(fmt),
        c.byref(count), c.byref(remaining), c.byref(data),
    )
    if result or not data:
        return None
    try:
        if fmt.value == 32:
            return list(c.cast(data, c.POINTER(c.c_ulong))[:count.value])
        return c.string_at(data, count.value).decode('utf-8', errors='replace')
    finally:
        x.XFree(data)


def find(window, depth=0):
    if depth > 5:
        return None
    name = prop(window, '_NET_WM_NAME') or prop(window, 'WM_NAME') or ''
    if (
        prop(window, '_NET_WM_PID') == [pid] and isinstance(name, str)
        and name.startswith('Computy ') and prop(window, 'WM_PROTOCOLS')
    ):
        return window
    root, parent = c.c_ulong(), c.c_ulong()
    children, count = c.POINTER(c.c_ulong)(), c.c_uint()
    if not x.XQueryTree(
        display, window, c.byref(root), c.byref(parent),
        c.byref(children), c.byref(count),
    ):
        return None
    try:
        for child in list(children[:count.value]):
            found = find(child, depth + 1)
            if found:
                return found
    finally:
        if children:
            x.XFree(children)
    return None


class Data(c.Union):
    _fields_ = [('b', c.c_char * 20), ('s', c.c_short * 10), ('l', c.c_long * 5)]


class Client(c.Structure):
    _fields_ = [
        ('type', c.c_int), ('serial', c.c_ulong), ('send_event', c.c_int),
        ('display', c.c_void_p), ('window', c.c_ulong),
        ('message_type', c.c_ulong), ('format', c.c_int), ('data', Data),
    ]


class Event(c.Union):
    _fields_ = [('client', Client), ('pad', c.c_long * 24)]


window = find(x.XDefaultRootWindow(display))
assert window, 'Computy main window not found in private test display'
event = Event()
event.client.type = 33
event.client.display = display
event.client.window = window
event.client.message_type = x.XInternAtom(display, b'WM_PROTOCOLS', 0)
event.client.format = 32
event.client.data.l[0] = x.XInternAtom(display, b'WM_DELETE_WINDOW', 0)
assert event.client.data.l[0] in prop(window, 'WM_PROTOCOLS')
x.XSendEvent.argtypes = [c.c_void_p, c.c_ulong, c.c_int, c.c_long, c.POINTER(Event)]
assert x.XSendEvent(display, window, 0, 0, c.byref(event))
x.XFlush.argtypes = [c.c_void_p]
x.XFlush(display)
x.XCloseDisplay(display)
