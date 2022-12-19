import os

path = "/home/seal/github/officejet_pro_6835/unpacker/segments/"

def condense_sections(files):
    for i in range(0, len(files)-1):
        if files[i][2] == files[i+1][1]:
            path1 = path+files[i][0]
            path2 = path+files[i+1][0]
            with open(path1, 'ab') as f1:
                with open(path2, 'rb') as f2:
                    file2_data = f2.read()
                    f1.write(file2_data)
                    files[i] = (files[i][0], files[i][1], files[i+1][2])
                    files.remove(files[i+1])

            print("Removing: " + path2)
            os.remove(path2)
            condense_sections(files)
            break

# Condense adjacent sections
files = []
for filename in os.listdir(path):
    file_stats = os.stat(path+filename)

    start = int("0x"+filename.split(".")[0], 16)
    end = start + file_stats.st_size
    files.append((filename, start, end))
files.sort()
condense_sections(files)
