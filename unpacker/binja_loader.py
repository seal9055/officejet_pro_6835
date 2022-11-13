import json

path = "printer/officejet_pro_6835/unpacker/" 
with open("segment_table", "r") as s:
    segments = json.load(s.read())
    for segment in segments:     
        with open(path + segment["name"], 'rb') as f:         
            data = f.read()         
            bv.parent_view.write(len(bv.parent_view), data)
            bv.add_user_segment(segment["load_addr"], len(data), 
                    len(bv.parent_view) - len(data), len(data), segment["perms"]) 
