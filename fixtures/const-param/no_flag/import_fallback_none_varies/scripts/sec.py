def units(report, section):
    return {report, section}


def deployed(report):
    return units(report, "Deployment units")


def deployed_again(report):
    return units(report, "Deployment units")
