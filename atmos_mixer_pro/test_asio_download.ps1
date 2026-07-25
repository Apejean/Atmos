Invoke-WebRequest -Uri "https://www.steinberg.net/asiosdk" -OutFile "asio.zip"
Expand-Archive -Path "asio.zip" -DestinationPath "."
