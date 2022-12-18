import os

path = "/home/seal/github/officejet_pro_6835/unpacker/segments/"

for filename in os.listdir(path):
    load_addr = int("0x"+filename.split(".")[0], 16)
    with open(path+filename, 'rb') as f:
        data = f.read()
        bv.parent_view.write(len(bv.parent_view), data)
        bv.add_user_segment(load_addr, len(data), 
                            len(bv.parent_view) - len(data), len(data), 7) 

bv.reanalyze()
