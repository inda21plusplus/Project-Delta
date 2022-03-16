from PIL import Image

im = Image.open('icon.png')  # Can be many different formats.
pixels = im.load()

outfile = open("icon.ppm", "w")
height, width = im.size

outfile.write(f"P3\n{width} {height}\n")

for y in range(height):
    for x in range(width):
        rbga = pixels[x, y]

        outfile.write(f"{rbga[2]} {rbga[0]} {rbga[1]} {rbga[3]}\n")
