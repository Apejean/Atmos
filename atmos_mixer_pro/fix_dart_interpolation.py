with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'r') as f:
    content = f.read()

bad_string = "mv.cameraOrbit = `${oldOrbit.theta}rad ${oldOrbit.phi}rad ${oldOrbit.radius}m`;"
good_string = r"mv.cameraOrbit = `${oldOrbit.theta}rad ${oldOrbit.phi}rad ${oldOrbit.radius}m`;"
good_string = good_string.replace('$', '\\$')

content = content.replace("mv.cameraOrbit = `${oldOrbit.theta}rad ${oldOrbit.phi}rad ${oldOrbit.radius}m`;", good_string)

with open('lib/features/exhibition/widgets/viewport_3d/dynamic_3d_room.dart', 'w') as f:
    f.write(content)
