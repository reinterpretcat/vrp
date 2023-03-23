# TODO 
# 1. should control how encoding decoding with quadkey is performed
#   - do not forget quadkey rotation
# 2. should prepare mask to use torch.masked_fill ind model generator logic

from . quadkey import *

import skbio
import numpy as np
import math
from pathlib import Path


class EncoderFeature():
    def __init__(self, encoding, tree) -> None:
        self.encoding = encoding
        #self.decoding = {v: k for k, v in encoding.items()}
        self.tree = tree


class Encoder:
    def __init__(self, spatial, temporal) -> None:
        self.spatial = spatial
        self.temporal = temporal

    def encode_activity(self, activity):
        idx = activity.idx
        return self.spatial.encoding[idx] + ' ' + self.temporal.encoding[idx]


def get_quad_tree_encoding(quad_tree, modifier_fn):
    return dict((point.data, modifier_fn(node.key.encode())) for node in find_children(quad_tree.root) if node.points for point in node.points)


def shift_code(s, offset):
    return ''.join(map(lambda c: chr(ord(c) + offset), s))


def rotate_pcoa(pcoa_dists, angle):
    def normalize(matrix):
        return (matrix - np.min(matrix)) / (np.max(matrix) - np.min(matrix))

    xs = pcoa_dists.samples['PC1']
    ys = pcoa_dists.samples['PC2']

    if angle != 0 and angle != 360:
        angle = angle * math.pi / 180.
        cos = math.cos(angle)
        sin = math.sin(angle)

        xs_new = xs * cos - ys * sin
        ys_new = xs * sin + ys * cos
    else:
        xs_new = xs
        ys_new = ys

    xs_new = normalize(xs_new)
    ys_new = normalize(ys_new)

    return np.stack([xs_new, ys_new], axis=1)


def get_spatial_features(distance_matrix, angles = None, visualize=False):
    # normalize distances inside [0, 1] range
    def normalize(matrix):
        return (matrix - np.min(matrix)) / (np.max(matrix) - np.min(matrix))

    distance_matrix = normalize(distance_matrix)
    pcoa_dists = skbio.stats.ordination.pcoa(distance_matrix)        

    if not angles:
        angles = np.arange(0, 360, 45)

    spatial_features = []
    for idx, angle in enumerate(angles):
        angle = angle * math.pi / 180.
        
        cos = math.cos(angle)
        sin = math.sin(angle)

        xs = pcoa_dists.samples['PC1']
        ys = pcoa_dists.samples['PC2']
        xs_new = xs * cos - ys * sin
        ys_new = xs * sin + ys * cos

        xs_new = normalize(xs_new)
        ys_new = normalize(ys_new)

        pcoa_coords = np.stack([xs_new, ys_new], axis=1)

        spatial_quad_tree = QuadTree(split_threshold = 1, rect_size = 1)
        for coord_idx, coord in enumerate(pcoa_coords):
            spatial_quad_tree.add_point(Point(coord[0], coord[1], coord_idx))

        spatial_quad_tree.subdivide()

        # visualize only first angle
        if idx == 0 and visualize:
            spatial_quad_tree.visualize()

        spatial_encoding = get_quad_tree_encoding(spatial_quad_tree, lambda s: s)

        spatial_features.append(EncoderFeature(spatial_encoding, spatial_quad_tree))

    return spatial_features



def get_temporal_feature(customers, visualize=False):
    customer_times = [time for customer in customers for time in customer.times]
    min_time, max_time = min(customer_times), max(customer_times)

    customer_times = [(np.array(customer.times) - min_time) / (max_time - min_time) for customer in customers]

    time_quad_tree = QuadTree(split_threshold = 1, lod_threshold= 4, rect_size = 1)

    for idx, time in enumerate(customer_times):
        time_quad_tree.add_point(Point(time[0], time[1], idx))

    time_quad_tree.subdivide()
    if visualize:
        time_quad_tree.visualize()
    
    temporal_encoding = get_quad_tree_encoding(time_quad_tree, lambda s: shift_code(s, offset = 4))

    return EncoderFeature(temporal_encoding, time_quad_tree)


def encode_routes(instance):
    # TODO add demand encoding
    spatial_features = get_spatial_features(instance.distance_matrix)
    temporal_feature = get_temporal_feature(instance.customers)

    encoded_routes_data = ''
    for spatial_feature in spatial_features:
        encoder = Encoder(spatial_feature, temporal_feature)

        encoded_routes = [[encoder.encode_activity(activity) for activity in route] for route in instance.routes]
        encoded_routes = [', '.join(route) + '.' for route in encoded_routes]
        encoded_routes = '\n'.join(encoded_routes)

        encoded_routes_data = encoded_routes_data + '\n\n' + encoded_routes

    return encoded_routes_data


def create_encoded_instances_on_disk(instances, cache_root_dir):
    encoded_instance_files = []
    
    for instance in instances:
        path = Path(cache_root_dir).joinpath(f"{instance.name}.enc")
        encoded_instance = encode_routes(instance)
        
        with open(path, 'w') as file:
            file.write(encoded_instance)

        encoded_instance_files.append(path)

    encoded_instance_files = [str(path) for path in encoded_instance_files]
    
    print(f"total encoded instance files: {len(encoded_instance_files)}")

    return encoded_instance_files


def get_encoded_instances_from_disk(encoded_instance_files):
    encoded_instance_data = ''
    
    for encoded_instance_file in encoded_instance_files:
        with open(encoded_instance_file, 'r') as file:
            content = file.read()
            encoded_instance_data = encoded_instance_data + content

    print(f"read total encoded instances: {len(encoded_instance_data)}")

    return encoded_instance_data
