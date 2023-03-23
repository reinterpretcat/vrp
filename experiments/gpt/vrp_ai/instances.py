import vrplib
import numpy as np
from pathlib import Path


# https://github.com/leonlan/VRPLIB

class VrpActivity:

    def __init__(self, idx, id, coord, demand, times, service):
        self.idx = idx
        self.id = id
        self.coord = coord
        self.demand = demand
        self.times = times
        self.service = service
    
    def __repr__(self):
        return f"VrpActivity[id={self.id}, coord={self.coord}, times={self.times}, service={self.service}]"


class VrpInstance:
    
    def __init__(self, name, instance_format, problem_path, solution_path = None):

        self.name = name

        # process problem definition
        self.problem = vrplib.read_instance(problem_path, instance_format)
        if instance_format == 'solomon':
            self._parse_solomon()
        elif instance_format == 'vrplib':
            self._parse_vrplib()
        else:
            raise NotImplementedError
        
        self.distance_matrix = np.round(self.problem['edge_weight'], 2)

        # process solution definition
        self.routes = []
        if solution_path:
            self.solution = vrplib.read_solution(solution_path)
            for route in self.solution['routes']:
                self.routes.append([self.customers[idx] for idx in [0, *route, 0]])

            self.cost = self.solution['cost']
        else:
            self.cost = None


    def has_solution(self):
        return self.cost != None


    def _parse_solomon(self):
        p = self.problem
        self.vehicles_size = p['vehicles']
        self.capacity = p['capacity']

        size = len(p['node_coord'])
        self.customers = [VrpActivity(idx, str(idx), p['node_coord'][idx], p['demand'][idx], p['time_window'][idx], p['service_time'][idx]) for idx in range(0, size)]


    def _parse_vrplib(self):
        p = self.problem
        self.vehicles_size = 0
        self.capacity = p['capacity']

        size = len(p['node_coord'])
        self.customers = [VrpActivity(idx, str(idx), p['node_coord'][idx], p['demand'][idx], [0, 0], 0) for idx in range(0, size)]


    def __repr__(self):
        return f"VrpInstance[name={self.name}, vehicles={self.vehicles_size}, customers={len(self.customers)}, routes={len(self.routes)}, cost={self.cost}]"


# download instances: might take some time
def download_instances(local_storage, instance_type):
    vrp_instances_path = Path(local_storage).joinpath(instance_type)
    vrp_instances_path.mkdir(parents=True, exist_ok=True)

    instance_names = vrplib.list_names(vrp_type=instance_type)
    print(f"total instances of {instance_type=}: {len(instance_names)}")
    
    for idx, instance_name in enumerate(instance_names):
        problem_file = vrp_instances_path.joinpath(f"{instance_name}.vrp")
        if not problem_file.is_file():
            vrplib.download_instance(instance_name, problem_file)

        vrp_solution = vrp_instances_path.joinpath(f"{instance_name}.sol")
        if not vrp_solution.is_file():
            vrplib.download_solution(instance_name, vrp_solution)
        
        if idx > 0 and idx % 20 == 0:
            print(f"processed {idx}")

    return instance_names


class VrpInstanceRegistry():

    def __init__(self, instance_root_dir) -> None:
        instance_types = ['cvrp', 'vrptw']
        
        self.types = instance_types
        self.root_dir = instance_root_dir
        self.names = dict([(instance_type, download_instances(instance_root_dir, instance_type)) for instance_type in instance_types])


    def get_instances(self, instance_type, instance_format):
        """
        Returns instances of given type
        """
        assert instance_type in self.types, f"unknown instance type: {instance_type}, should be one from {self.types}"

        def get_path(name, ext):
            return Path(self.root_dir).joinpath(instance_type, f"{name}.{ext}")

        return [VrpInstance(name, instance_format, get_path(name, 'vrp'), get_path(name, 'sol')) for name in self.names[instance_type]]


    def size(self):
        """
        Returns a total amount of instances of all types
        """
        return sum(len(value) for _, value in self.names.items())


    def __repr__(self):
        return f"VrpInstanceRegistry[types={self.types}, size={self.size()}]"