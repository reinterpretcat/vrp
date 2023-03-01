import matplotlib.pyplot as plt
import matplotlib.patches as patches

# https://jrtechs.net/data-science/implementing-a-quadtree-in-python


class Point():
    def __init__(self, x, y, data):
        self.x = x
        self.y = y
        self.data = data

    def __repr__(self):
        return f"Point(x={self.x}, y={self.y}, data={self.data})"


class Rect():
    def __init__(self, x, y, w, h):
        self.x = x
        self.y = y
        self.w = w
        self.h = h

    def __repr__(self):
        return f"Rect(x={self.x}, y={self.y}, w={self.w}, h={self.h})"


class QuadKey():
    def __init__(self, lod, x, y):
        self.lod = lod
        self.x = x
        self.y = y

    def encode(self):
        code = ''
        for i in range(self.lod, 0, -1):
            digit = ord('0')
            mask = 1 << (i - 1)
            if (self.x & mask) != 0:
                digit += 1

            if (self.y & mask) != 0:
                digit += 1
                digit += 1

            code += chr(digit)

        return code

    @staticmethod
    def decode(quad_key):
        x, y = 0, 0
        lod = len(quad_key)
        for i in range(lod, 0, -1):
            mask = 1 << (i - 1)

            match quad_key[lod - i]:
                case '0':
                    pass
                case '1':
                    x |= mask
                case '2':
                    y |= mask
                case '3':
                    x |= mask
                    y |= mask
                case _:
                    raise (f"Invalid QuadKey digit sequence: {quad_key}")

        return QuadKey(lod, x, y)

    def __repr__(self):
        return f"QuadKey(lod={self.lod} , x={self.x}, y={self.y})"


class Node():
    def __init__(self, rect, key, points):
        self.rect = rect
        self.key = key
        self.points = points
        self.children = []

    def __repr__(self):
        return f"Node(rect={self.rect}, key={self.key}, children={len(self.children)}, points={len(self.points)})"


def recursive_subdivide(node, split_threshold, lod_threshold):
    if len(node.points) <= split_threshold:
        return

    if node.key.lod > lod_threshold:
       return

    w_, h_ = float(node.rect.w / 2), float(node.rect.h / 2)
    x, y = node.rect.x, node.rect.y

    children = []
    for x0, y0 in [(x, y), (x, y + h_), (x + w_, y), (x + w_, y + h_)]:
        rect = Rect(x0, y0, w_, h_)
        key = QuadKey(node.key.lod + 1, int(x0 / w_), int(y0 / h_))
        points = get_points(rect, node.points)
        child = Node(rect, key, points)
        recursive_subdivide(child, split_threshold, lod_threshold)
        children.append(child)

    node.children = children


def get_points(rect, points):
    pts = []
    x_offset = rect.x + rect.w
    y_offset = rect.y + rect.h
    for point in points:
        if point.x >= rect.x and point.x <= x_offset and point.y >= rect.y and point.y <= y_offset:
            pts.append(point)
    return pts


def find_children(node):
    if not node.children:
        return [node]
    else:
        children = []
        for child in node.children:
            children += (find_children(child))
    return children


class QuadTree():

    def __init__(self, split_threshold, rect_size, lod_threshold = 10):
        self.split_threshold = split_threshold
        self.lod_threshold = lod_threshold
        self.points = []
        self.root = Node(Rect(0, 0, rect_size, rect_size),
                         QuadKey(0, 0, 0), self.points)

    def add_point(self, point):
        self.points.append(point)


    def subdivide(self):
        recursive_subdivide(self.root, self.split_threshold, self.lod_threshold)
        self.index = dict((node.key.encode(), node.points) for node in find_children(self.root) if node.points)


    def visualize(self):
        plt.figure(figsize=(12, 8))
        plt.title("Quadtree")

        children = find_children(self.root)
        print("Number of segments: %d" % len(children))
        areas = set()
        for child in children:
            areas.add(child.rect.w * child.rect.h)

        print("Minimum segment area: %.3f units" % min(areas))
        for child in children:
            rect = child.rect
            plt.gcf().gca().add_patch(patches.Rectangle(
                (rect.x, rect.y), rect.w, rect.w, fill=False))
        x = [point.x for point in self.points]
        y = [point.y for point in self.points]
        plt.plot(x, y, 'ro')
        plt.show()


    def __repr__(self):
        return f"QuadTree(points={len(self.points)})"
