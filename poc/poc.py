"""Proof of concept to open and preview an image/video in terminal"""
import os
from PIL import Image, ImageDraw, ImageFont
import numpy as np
import cv2
import time
import threading

# Open the video file for reading
cap = cv2.VideoCapture("poc/bad_apple.mp4")
# cap = cv2.VideoCapture(0)

cap.set(cv2.CAP_PROP_FRAME_WIDTH, 960)
cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 720)

IMG = Image.open("poc/homer2.jpg")


def draw_image(x_size, y_size, filter_strength=1, IMG=IMG):
    # ASCII characters to use for the image, sorted by decreasing intensity
    # ASCII_CHARS = " .'`^\",:;Il!i~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$"
    ASCII_CHARS = " .:-=+*#%@"
    CHAR_HEIGHT = 2 * filter_strength
    CHAR_WIDTH = 1 * filter_strength
    # estimate character size in pixels

    # Resize the image to a smaller size to speed up the process
    # maintain aspect ratio
    WIDTH, HEIGHT = IMG.size
    y_scale = (
        HEIGHT / (y_size * CHAR_HEIGHT) + 1e-3
    )  # Add a small correction factor or there will be errors down the line
    x_scale = (
        WIDTH / (x_size * CHAR_WIDTH) + 1e-3
    )  # Add a small correction factor or there will be errors down the line
    image = IMG.resize(
        (int(WIDTH // x_scale), int(HEIGHT // y_scale)), Image.Resampling.NEAREST
    )

    # image = image.load()

    # Convert the image to grayscale
    image = image.convert("L")

    # Convert the image to a numpy array
    image_np = np.array(image)

    # Create a new numpy array, 3 times smaller than the image
    # This will be used to store the ASCII characters
    ascii_image = np.zeros(
        (image_np.shape[0] // CHAR_HEIGHT, image_np.shape[1] // CHAR_WIDTH)
    )

    # for each 3x3 group of pixels, find the average intensity
    # and use it to find the corresponding ASCII character
    for i in range(ascii_image.shape[0]):
        for j in range(ascii_image.shape[1]):
            ascii_image[i, j] = image_np[CHAR_HEIGHT * i, CHAR_WIDTH * j]
            # ascii_image[i, j] = np.mean(
            #     image_np[
            #         CHAR_HEIGHT * i : CHAR_HEIGHT * i + CHAR_HEIGHT,
            #         CHAR_WIDTH * j : CHAR_WIDTH * j + CHAR_WIDTH,
            #     ]
            # )
    # Build the ASCII image
    ascii_string = "\n".join(
        [
            "".join([ASCII_CHARS[int(pixel * len(ASCII_CHARS) / 256)] for pixel in row])
            for row in ascii_image
        ]
    )
    return ascii_string


import queue

# Create a thread-safe queue to pass frames between threads
frame_queue = queue.Queue()


# Define a function to read frames from the camera in a separate thread
def read_frames():
    cnt = 0
    while True:
        # print("ASDASD\n\r")
        # Read the current frame
        ret, frame = cap.read()
        # cap.set(cv2.CAP_PROP_POS_FRAMES, cnt * 60)
        # Check if the frame was successfully read
        if ret:
            # print("SUCCESS\n\r")
            # Add the frame to the queue
            # if cnt % 60 == 0:
            frame_queue.put(frame)
            cnt += 1
        # Wait for a short time to avoid excessive CPU usage
        time.sleep(0.04)


# Create a new thread for reading frames
t = threading.Thread(target=read_frames)
t.daemon = True
t.start()


import curses

from colorama import init, Fore, Back, Style


def main(stdscr):
    # Clear screen
    stdscr.clear()

    # Get initial terminal size
    old_height, old_width = stdscr.getmaxyx()

    init()
    first_draw = True
    fs = 1
    # global last_frame
    while True:
        # Check if terminal has been resized
        height, width = stdscr.getmaxyx()
        # if height != old_height or width != old_width or first_draw:
        # Read the current frame
        # if last_frame is not None:
        # frame = np.array(frame)
        pixel_width = curses.COLS // width
        pixel_height = curses.LINES // height
        # Terminal has been resized, redraw
        old_height, old_width = height, width
        # If there is a new frame available, draw it on the screen
        try:
            frame = frame_queue.get_nowait()
            stdscr.clear()
            # stdscr.refresh()

            astr = draw_image(
                width, height, filter_strength=fs, IMG=Image.fromarray(frame)
            )
            stdscr.addstr(0, 0, astr)
            stdscr.refresh()
        except queue.Empty:
            time.sleep(0.02)
            pass

        first_draw = False
        # Print output to screen
        # stdscr.addstr(height // 2, width // 2, "Hello, World!")

        # Initialize colorama

        # Define the string to print
        # hello_world = "Hello, world!"

        # Reset the color
        # print(Style.RESET_ALL)

        # Refresh screen

        # Wait for user input
        # x = stdscr.getch()
        # if x is not None:
        # first_draw = True
        # time.sleep(1)


if __name__ == "__main__":
    curses.wrapper(main)

# import curses


# def main(stdscr):
#     # Initialize colors
#     curses.start_color()
#     curses.use_default_colors()

#     # Define the string to print
#     hello_world = "Hello, world!"

#     # Define a list of foreground colors
#     fg_colors = [
#         (255, 0, 0),  # red
#         (0, 255, 0),  # green
#         (0, 0, 255),  # blue
#         (255, 255, 0),  # yellow
#         (255, 0, 255),  # magenta
#         (0, 255, 255),  # cyan
#     ]

#     # Define color pairs
#     for i, color in enumerate(fg_colors):
#         r, g, b = color
#         curses.init_color(
#             i + 1, int(r / 255 * 1000), int(g / 255 * 1000), int(b / 255 * 1000)
#         )
#         curses.init_pair(i + 1, i + 1, -1)

#     # Print each letter of the string in a different color
#     hws = ""
#     for i, letter in enumerate(hello_world):
#         color_pair = curses.color_pair((i % len(fg_colors)) + 1)
#         hws += chr(color_pair) + letter

#     while True:
#         # Check if terminal has been resized
#         height, width = stdscr.getmaxyx()
#         if height != 0 or width != 0:
#             # Terminal has been resized
#             stdscr.clear()
#             stdscr.addstr(0, 0, f"New size: {height} rows x {width} columns")

#         # Print output to screen
#         for i, letter in enumerate(hello_world):
#             color_pair = curses.color_pair((i % len(fg_colors)) + 1)
#             stdscr.addstr(
#                 height // 2, width // 2 + i - len(hello_world) // 2, letter, color_pair
#             )

#         # Refresh screen
#         stdscr.refresh()

#         # Wait for user input
#         stdscr.getch()


# if __name__ == "__main__":
#     curses.wrapper(main)
