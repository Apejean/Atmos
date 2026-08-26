import re

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
    content = f.read()

# I see the RenderFlex overflow error is back!
# It happened on line 1552. Let's fix the AppBar title by changing `Row` to `Wrap` or adding `Expanded` to its children.
# Or better, wrap the Row inside `SingleChildScrollView(scrollDirection: Axis.horizontal, child: Row(...))` like I tried before, 
# but make sure we don't cause the `RenderSingleChildViewport` error.
# The `RenderSingleChildViewport` error usually happens when `SingleChildScrollView` is not constrained vertically, but `AppBar` title is constrained.
# Another way to fix RenderFlex overflow in an AppBar is to simply wrap the Row in an `Expanded` if it's inside another Row, 
# OR just use a `ListView` with horizontal scrolling, OR change the buttons.

# Wait, let's just make the `Row` scrollable by using `SingleChildScrollView` but properly wrapped.
content = content.replace(
    '''        title: Row(
          children: [
            const Text('Exhibition Canvas',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
            const SizedBox(width: 16),
            const Text('|', style: TextStyle(color: Colors.white24)),
            const SizedBox(width: 16),''',
    '''        title: SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Row(
            children: [
              const Text('Exhibition Canvas',
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
              const SizedBox(width: 16),
              const Text('|', style: TextStyle(color: Colors.white24)),
              const SizedBox(width: 16),'''
)

# And add the closing parenthesis for SingleChildScrollView at the end of the title's Row.
content = content.replace(
    '''],
        ),
        backgroundColor: Colors.black,''',
    '''],
          ),
        ),
        backgroundColor: Colors.black,'''
)

with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
    f.write(content)

