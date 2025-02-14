import json
import os
import random
import requests
from fastapi import FastAPI, Response
from datetime import datetime, date, timedelta, timezone
import logging
from pydantic import BaseModel


policy_details = """
apiVersion: policy.open-cluster-management.io/v1
kind: Policy
metadata:
  name: {policy_name}
  namespace: {namespace}
  labels:
    category: "Configuration-Management"  # Fixed label issue
spec:
  remediationAction: {remediation_action}  # Change to 'inform' if needed
  disabled: false
  policy-templates:
    - objectDefinition:
        apiVersion: policy.open-cluster-management.io/v1
        kind: ConfigurationPolicy
        metadata:
          name: enforce-default-namespace
        spec:
          remediationAction: {remediation_action}
          severity: low
          namespaceSelector:
            include: ["{namespace}"]
          object-templates:
            - complianceType: musthave
              objectDefinition:
                apiVersion: v1
                kind: Namespace
                metadata:
                  name: archtest
                  labels:
                    environment: production

---
# PlacementRule Definition
apiVersion: apps.open-cluster-management.io/v1
kind: PlacementRule
metadata:
  name: placement-local-cluster
  namespace: {namespace}
spec:
  clusterSelector:
    matchLabels:
      name: local-cluster

---
# PolicyBinding Definition
apiVersion: policy.open-cluster-management.io/v1
kind: PlacementBinding
metadata:
  name: binding-default-namespace-policy
  namespace: {namespace}
placementRef:
  apiGroup: apps.open-cluster-management.io  # Fixed missing apiGroup
  kind: PlacementRule
  name: placement-local-cluster
subjects:
  - name: {policy_name}
    kind: Policy
    apiGroup: policy.open-cluster-management.io
"""

logger = logging.getLogger("uvicorn.error")
logger.setLevel(logging.INFO)

app = FastAPI()


@app.get("/healthz")
async def healthz():
    return {"status": "ok"}


class PolicyPut(BaseModel):
    policy_name: str
    remediationAction: str
    namespace: str


@app.post("/create_policy")
async def create_policy(req: PolicyPut, res: Response):
    logger.info(
        f"Creating policy: {req.policy_name} in namespace: {req.namespace}, remediationAction: {req.remediationAction}"
    )
    policy = policy_details.format(
        policy_name=req.policy_name,
        namespace=req.namespace,
        remediation_action=req.remediationAction,
    )
    logger.info(f"Policy: {policy}")
    endpoint = f"{os.getenv('ACM_SERVER', 'http://localhost:8080')}/apis/policy.open-cluster-management.io/v1/namespaces/{req.namespace}/policies"
    headers = {"content-type": "application/yaml", "host": "localhost"}
    logger.info(f"Endpoint: {endpoint}, headers: {headers}")
    resp = requests.post(endpoint, data=policy, headers=headers)
    resp_text = resp.text
    logger.info(f"Response: {resp_text}")

    return {"status: ": resp_text}
