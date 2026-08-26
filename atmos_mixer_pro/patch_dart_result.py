with open("lib/features/dashboard/screens/dashboard_screen.dart", "r") as f:
    content = f.read()

content = content.replace(
    "final chId = res.channel + 1;",
    "final chId = res.channel.toInt() + 1;"
)

# And fix the Uint64List error from frb by just using List<BigInt> instead of Uint64List? Wait, in Dart Uint64List from typed_data doesn't hold BigInt. FRB uses its own Uint64List or we can just use `List<BigInt>` if that's what FRB takes, wait, let's see. 
# FRB uses Uint64List from flutter_rust_bridge. Let's just import it or just use generic list casting.
