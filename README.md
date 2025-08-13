# INFO
Migrated to GitLab:
https://gitlab.com/kazam0180/codlinux


# codlinux
[![Rust Release](https://github.com/coyoteclan/codlinux/actions/workflows/rust.yml/badge.svg)](https://github.com/coyoteclan/codlinux/actions/workflows/rust.yml)

Unlike CoD 2, CoD 1/UO require some environment variables to be set to run through wine (see [this](https://appdb.winehq.org/objectManager.php?sClass=version&iId=36969)).
To run the games, you need a script. But using scripts is a little inconvenient. This wrapper allows you to run these games easily and also sets up default app (itself) for opening `iw1x://` uri scheme.

## Notes
- You can remove the remembered game from ``codlinux_conf/codlinux.cfg``. For example,<br>
  <img width="330" height="64" alt="Screenshot_2025-08-04_19-01-02" src="https://github.com/user-attachments/assets/597cb1e0-b9db-47bf-b4dc-eb5068967167" /><br>
- You can only check for updates 60 times per hour
- Press **ESC** if "More Options" menu doesn't close. This is a gtk4 issue.

## Todo
- Add an option to create a launcher of individual games

## Screenshots

### When run from CoD (1.1) folder
<img width="404" height="390" alt="Screenshot_2025-08-04_18-31-26" src="https://github.com/user-attachments/assets/18016fe9-8182-456c-a854-28ce309aa734" />


### When run from UO folder
<img width="404" height="545" alt="Screenshot_2025-08-04_18-33-16" src="https://github.com/user-attachments/assets/dff754ed-0499-4e8a-80af-5f017334e18e" />


### Updating
<img width="304" height="134" alt="Screenshot_2025-08-04_18-31-46" src="https://github.com/user-attachments/assets/d59f36d5-4b6c-4e53-81e5-7ba97ee50f17" />
<img width="304" height="134" alt="Screenshot_2025-08-04_18-32-06" src="https://github.com/user-attachments/assets/4dc510d3-ec43-4c25-b143-8b46c40ee13c" />
