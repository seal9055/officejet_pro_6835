path = "/home/seal/printer/officejet_pro_6835/unpacker/" 
files = [
        {"name": "cromtext", "load_addr": 0x26779548, "perms": 0x5},
        {"name": "cromdata", "load_addr": 0x26779860, "perms": 0x6},
        {"name": "crom_ro_data", "load_addr": 0x267795cc, "perms": 0x6},
        {"name": "crom_nc_data", "load_addr": 0x267797b4, "perms": 0x6},
        {"name": "crom_module", "load_addr": 0x2677f166c, "perms": 0x7},
        {"name": "crom_fs", "load_addr": 0x26779588, "perms": 0x6},
        {"name": "crom_fs_objs", "load_addr": 0x267795a8, "perms": 0x6},
]

for file in files:     
    with open(path + file["name"], 'rb') as f:         
        data = f.read()         
        bv.parent_view.write(len(bv.parent_view), data)
        bv.add_user_segment(file["load_addr"], len(data), len(bv.parent_view)-len(data),
                len(data), file["perms"]) 
