import csv
# pip install matplotlib
import matplotlib.pyplot as pyplot
import os
import sys

# Data
CLOCK = []
SAMPLE = []

path = "samples.txt"

if not os.path.exists(path):
    print("Sample file does not exist: \"", path,"\"")
    sys.exit(1)

with open(path, "r") as file:
    r = csv.reader(file, delimiter=' ')
    for row in r:
        for value in row:
            if value:
                SAMPLE.append(float(value))

CLOCK = range(len(SAMPLE))

print("Given: ", len(SAMPLE), " samples.")

# Plotting
pyplot.figure(figsize=(8, 5))
pyplot.plot(CLOCK, SAMPLE, marker='o', color='g')
pyplot.title('Audio Sample Oscilloscope')
pyplot.xlabel('Clock Tick')
pyplot.ylabel('Sample Value')
pyplot.grid()
pyplot.show()