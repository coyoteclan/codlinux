###### toplink

# codlinux
Unlike CoD 2, CoD 1/UO require some environment variables to be set to run through wine (see [this](https://appdb.winehq.org/objectManager.php?sClass=version&iId=36969)).
To run the games, you need a script. But using scripts is a little inconvenient. This wrapper allows you to run these games easily and also sets up default app (itself) for opening `iw1x://` uri scheme.

## Notes
- You can remove the remembered game from codlinux.cfg. For example,
  ![Screenshot_2025-04-21_16-41-01](https://github.com/user-attachments/assets/144e615d-1fc0-484e-8730-213c59df2699)
- For safety reasons, the wine prefix folder should be created **before** setting in CoDLinux
- You can only check for updates 60 times per hour

## Todo
- Verify the game exe (version)

## Screenshots

### When run from CoD (1.1) folder
![Screenshot_2025-04-20_19-04-46](https://github.com/user-attachments/assets/4d7d0369-080d-43d4-b3ac-17964b071f4a)

### When run from UO folder
![Screenshot_2025-04-20_19-05-14](https://github.com/user-attachments/assets/53c11e20-d3c7-43de-b923-48780eff56ce)

### Updating
![Screenshot_2025-04-21_16-45-16](https://github.com/user-attachments/assets/ab2a383a-88af-40e3-a8ee-52ff40ba8336)
![Screenshot_2025-04-21_16-45-26](https://github.com/user-attachments/assets/abcd45e6-9798-4cdd-a142-0e04dd183ccb)
