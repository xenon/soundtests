import csv
# pip install plotty
import plotly.graph_objects as go
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
fig = go.Figure()
fig.add_trace(go.Scatter(x=list(CLOCK), y=SAMPLE,
                    mode='lines+markers',
                    name='lines+markers'))
#fig = px.scatter(x=CLOCK, y=SAMPLE, mode='lines+markers')
# Update layout to include grid lines
fig.update_layout(title='Audio Sample Oscilloscope',
                  xaxis_title='Clock Tick',
                  yaxis_title='Sample Value',
                  showlegend=False)

# Show the figure
fig.show()
