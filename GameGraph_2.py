import json
import matplotlib.pyplot as plt
import os




# Load color data from JSON file
with open('TeamSettings.json', 'r') as file:
    data = json.load(file)
    colors = data['colors']  # List of RGB lists (e.g., [[0,0,255], ...])

# Convert JSON colors to normalized RGBA tuples (with alpha=0.75)
colorArr = [
    (r/255.0, g/255.0, b/255.0, 0.75)
    for r, g, b in colors
]

colorArr_light = [
    (r/255.0, g/255.0, b/255.0, 0.25)
    for r, g, b in colors
]

os.chdir("Game Data")




def tank_scatter(file, axis, xlabel, ylabel, dots=False):
    with open(file) as f:
        data = json.load(f)
        
        value_array = []
        team_array = []
        time_array = []
        for i in range(len(data)):
           for j in range(len(data[i])):
               time_array.append(i)
               team_array.append(data[i][j][0])
               value_array.append(data[i][j][1])

        array_x = time_array
        array_y = value_array
        axis.scatter(array_x, array_y, c=[colorArr[int(team)] for team in team_array], s=1 if dots else 5)
        axis.set_xlabel(xlabel)
        axis.set_ylabel(ylabel)



fig, ax = plt.subplots(1)
tank_scatter("Tank_Age_Team_Time.json", ax, "Time", "Tank Age (ticks)", dots=True)
plt.show()

fig, ax = plt.subplots(1)
tank_scatter("Tank_Kills_Team_Time.json", ax, "Time", "Tank Kills", dots=True)
plt.show()

