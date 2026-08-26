with open("lib/features/dashboard/screens/dashboard_screen.dart", "r") as f:
    content = f.read()

content = content.replace("final chId = res.channel + 1;", "final chId = res.channel.toInt() + 1;")
content = content.replace("Uint64List.fromList(speakerChannels.map((e) => e.toInt()).toList())", "speakerChannels") # revert that if we didn't need it because it seems speakerChannels is already Uint64List in frb?
# Wait! In the error message:
# `The argument type 'int' can't be assigned to the parameter type 'BigInt'.` on 1072. That is `res.channel + 1` where res.channel is BigInt and 1 is int.

with open("lib/features/dashboard/screens/dashboard_screen.dart", "w") as f:
    f.write(content)
