# Jocker example

## Usage

- Start processes using the default stack : `jocker start`
- Show processes state : `jocker ps`
- Stream processes logs and prefix lines : `jocker logs -p -f`
- Stop single process : `jocker stop eris`
- Use a different stack : `jocker --stack full ps`

Variables in a `.env` file will be picked up if present.
